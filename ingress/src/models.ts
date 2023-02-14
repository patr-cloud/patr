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

export type RoutingValue = Array<UrlType>;

export type UrlType =
    | {
        path: string,
        type: "proxyDeployment";
        deploymentId: string;
        port: number;
    }
    | {
        path: string,
        type: "proxyStaticSite";
        staticSiteId: string;
    }
    | {
        path: string,
        type: "proxyUrl";
        url: string;
    }
    | {
        path: string,
        type: "redirect";
        url: string;
        permanent: boolean;
    };
