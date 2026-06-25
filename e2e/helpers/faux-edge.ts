import { mkdtempSync, rmSync } from 'node:fs';
import { request as httpsRequest } from 'node:https';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { execa } from 'execa';
import type { DindHandle } from '@/helpers/dind';
import { waitFor } from '@/helpers/process';

const SERVICE_NAME = 'faux-edge';
const INGRESS_NETWORK = 'patr-ingress-network';
const CADDYFILE = join(import.meta.dirname, '..', 'config', 'faux-edge.Caddyfile');

// A stand-in for the production Cloudflare edge + cloudflared tunnel.
//
// In production a private runner's traffic arrives like this:
//
//   user --HTTPS--> Cloudflare edge --tunnel--> cloudflared --HTTP--> patr-ingress (Caddy)
//                   (terminates TLS)            (catch-all rule:
//                                                http://patr-ingress:80)
//
// The faux edge is the same shape minus the dial-out to Cloudflare: a one-replica
// Caddy service that terminates TLS locally with a throwaway self-signed cert and
// forwards plain HTTP to the same `http://patr-ingress:80` target. That reproduces
// the one failure mode the tunnel hop can introduce — TLS terminated upstream, the
// request forwarded to the origin as plain HTTP — so redirect loops and other
// proxy-interaction bugs actually surface in e2e, which hitting Caddy directly
// can't show.
//
// It runs as a swarm *service* on purpose: the runner's ingress overlay is not
// `attachable`, so a standalone `docker run` container can't join it, but a swarm
// service can. That keeps the runner untouched.
export class FauxEdge implements AsyncDisposable {
  private constructor(private readonly dind: DindHandle) {}

  static async deploy(dind: DindHandle): Promise<FauxEdge> {
    const env = { ...process.env, DOCKER_HOST: dind.dockerHost };
    const work = mkdtempSync(join(tmpdir(), 'patr-e2e-faux-edge-'));

    try {
      // A throwaway self-signed cert the edge presents for every connection. Not
      // `tls internal` — that only issues certs for names Caddy serves, and the
      // port-only `:443` site has none, so arbitrary deployment hostnames fail
      // the handshake. The test client doesn't verify the cert.
      const crt = join(work, 'edge.crt');
      const key = join(work, 'edge.key');
      await execa(
        'openssl',
        [
          'req',
          '-x509',
          '-newkey',
          'rsa:2048',
          '-nodes',
          '-keyout',
          key,
          '-out',
          crt,
          '-days',
          '3650',
          '-subj',
          '/CN=onpatr.cloud',
          '-addext',
          'subjectAltName=DNS:*.onpatr.cloud,DNS:onpatr.cloud',
        ],
        { env },
      );

      // The runner creates the ingress overlay eagerly on init, but the service
      // create still races it — wait until the network exists.
      await waitFor(
        async () => {
          const res = await execa('docker', ['network', 'inspect', INGRESS_NETWORK], {
            env,
            reject: false,
          });
          return res.exitCode === 0;
        },
        { timeoutMs: 30_000, label: `${INGRESS_NETWORK} present` },
      );

      await execa('docker', ['config', 'create', 'faux_edge_caddyfile', CADDYFILE], { env });
      await execa('docker', ['config', 'create', 'faux_edge_crt', crt], { env });
      await execa('docker', ['config', 'create', 'faux_edge_key', key], { env });

      await execa(
        'docker',
        [
          'service',
          'create',
          '--name',
          SERVICE_NAME,
          '--network',
          INGRESS_NETWORK,
          // Publish via the swarm routing mesh on 8080 — free now that private
          // runners don't publish the ingress. The DinD maps 8080 → host
          // ingressPort, so the test reaches the edge over HTTPS there.
          '--publish',
          'published=8080,target=443',
          '--config',
          'source=faux_edge_caddyfile,target=/etc/caddy/Caddyfile',
          '--config',
          'source=faux_edge_crt,target=/etc/caddy/edge.crt',
          '--config',
          'source=faux_edge_key,target=/etc/caddy/edge.key',
          'caddy:2-alpine',
        ],
        { env },
      );

      // Wait until the edge answers TLS on the published port. Any HTTP response
      // (even a 502 while a deployment warms up) means Caddy is serving.
      await waitFor(() => probe(dind.ingressPort), {
        timeoutMs: 30_000,
        label: `${SERVICE_NAME} serving on :${dind.ingressPort}`,
      });

      return new FauxEdge(dind);
    } finally {
      rmSync(work, { recursive: true, force: true });
    }
  }

  async [Symbol.asyncDispose](): Promise<void> {
    const env = { ...process.env, DOCKER_HOST: this.dind.dockerHost };
    await execa('docker', ['service', 'rm', SERVICE_NAME], { env, reject: false }).catch(() => {});
  }
}

function probe(port: number): Promise<boolean> {
  return new Promise((resolve) => {
    const req = httpsRequest(
      { host: '127.0.0.1', port, path: '/', method: 'GET', rejectUnauthorized: false },
      (res) => {
        res.resume();
        resolve(true);
      },
    );
    req.on('error', () => resolve(false));
    req.end();
  });
}
