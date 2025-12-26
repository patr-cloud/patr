import { useNavigate } from "@solidjs/router";
import {
  createMemo,
  createResource,
  createSignal,
  ErrorBoundary,
  Suspense,
  Show,
} from "solid-js";
import { FiCheck, FiCopy, FiAlertCircle } from "solid-icons/fi";
import {
  PageContainer,
  PageContainerBody,
  PageContainerHead,
  Table,
  Button,
  ButtonVariant,
  useToast,
} from "~/components";
import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";
import InfoPopup from "~/components/info-popup";

// Type definitions based on API bindings
type WorkspaceDomain = {
  id: string;
  name: string;
  nameserverType: string;
  isVerified: boolean;
};

type GetDomainsForWorkspaceResponse = {
  domains: WorkspaceDomain[];
};

const CopyButton = (props: { text: string }) => {
  const [copied, setCopied] = createSignal(false);

  const handleCopy = async (e: MouseEvent) => {
    e.stopPropagation();
    try {
      await navigator.clipboard.writeText(props.text);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (error) {
      console.error("Failed to copy:", error);
    }
  };

  return (
    <button
      onClick={handleCopy}
      class="ml-2 p-1 rounded hover:bg-white/10 transition-colors"
      title={copied() ? "Copied!" : "Copy ID"}
    >
      {copied() ? (
        <FiCheck size={14} class="text-gray-400" />
      ) : (
        <FiCopy size={14} class="text-gray-400" />
      )}
    </button>
  );
};

const VerificationIcon = (props: { domain: WorkspaceDomain }) => {
  if (props.domain.isVerified) {
    return null;
  }

  return (
    <InfoPopup
      triggerIcon={() => <FiAlertCircle size={16} class="text-yellow-500" />}
      title="Verification Instructions"
      content={(close) => (
        <>
          <p class="text-gray-300 text-sm mb-2">
            To verify your domain, add the following DNS record:
          </p>
          <div class="bg-black/30 p-2 rounded text-xs text-gray-400 mb-2">
            <p>Type: TXT</p>
            <p>Name: _patr-verification</p>
            <p>Value: {props.domain.id}</p>
          </div>
          <p class="text-gray-400 text-xs">
            After adding the DNS record, it may take up to 48 hours to
            propagate.
          </p>
        </>
      )}
    />
  );
};

const ListDomainsPage = () => {
  const [authState] = useAuthState();
  const [workspaceId] = useLastWorkspaceId();
  const navigate = useNavigate();
  const toast = useToast();

  const fetchParams = createMemo(() => {
    return [authState(), workspaceId()] as const;
  });

  const [domains] = createResource(fetchParams, async ([auth, wsId]) => {
    if (!wsId || !auth || auth.type !== "LoggedIn") {
      return { domains: [] };
    }

    const response = await httpRequest<GetDomainsForWorkspaceResponse>(
      `${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/domain`,
      {
        method: "GET",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${auth.accessToken}`,
        },
      }
    );

    if (!response.ok) {
      console.error("Failed to fetch domains:", response.data.error);
      toast("Failed to fetch domains", "error");
      return { domains: [] };
    }

    console.log("Fetched domains:", response.data);
    return { domains: response.data.domains || [] };
  });

  return (
    <PageContainer>
      <PageContainerHead
        title="Domains"
        subTitle="All Domains"
        actions={() => (
          <div class="ml-auto">
            <Button
              variant={ButtonVariant.Contained}
              onClick={() => navigate("/domains/new")}
            >
              Add Domain
            </Button>
          </div>
        )}
      />
      <PageContainerBody>
        <ErrorBoundary
          fallback={(err, reset) => (
            <div>
              <p>Error loading domains: {err.message}</p>
              <button onClick={reset}>Retry</button>
            </div>
          )}
        >
          <Suspense fallback={<div>Loading domains...</div>}>
            <Table
              column_grids={["flex-3", "flex-3", "flex-2", "flex-2"]}
              rows={domains()?.domains || []}
              headings={["Domain ID", "Domain Name", "Type", "Verified"]}
              renderRow={(item) => (
                <tr
                  onClick={() => navigate(`/domains/${item.id}`)}
                  class="table-row"
                >
                  <td class="flex-3 flex items-center justify-center">
                    <span class="truncate">{item.id}</span>
                    <CopyButton text={item.id} />
                  </td>
                  <td class="flex-3 flex items-center justify-center">
                    {item.name}
                  </td>
                  <td class="flex-2 flex items-center justify-center">
                    {item.nameserverType}
                  </td>
                  <td class="flex-2 flex items-center justify-center">
                    <div class="flex items-center gap-2">
                      {item.isVerified ? (
                        <span class="text-green-500">✓ Verified</span>
                      ) : (
                        <>
                          <span class="text-yellow-500">Not Verified</span>
                          <VerificationIcon domain={item} />
                        </>
                      )}
                    </div>
                  </td>
                </tr>
              )}
            />
          </Suspense>
        </ErrorBoundary>
      </PageContainerBody>
    </PageContainer>
  );
};

export default ListDomainsPage;
