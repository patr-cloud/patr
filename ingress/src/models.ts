export type RoutingValue = Map<string, UrlType>;

export type UrlType =
    | {
          type: "proxyDeployment";
          deploymentId: string;
          port: number;
      }
    | {
          type: "proxyStaticSite";
          staticSiteId: string;
      }
    | {
          type: "proxyUrl";
          url: string;
      }
    | {
          type: "redirect";
          url: string;
          permanent: boolean;
      };

export type DeploymentValue =
    | ("created" | "stopped" | "deleted")
    | {
          running?: {
              regionId?: string;
              ports?: number[];
          };
      };

export type StaticSiteValue =
    | ("created" | "stopped" | "deleted")
    | {
          serving?: string;
      };
