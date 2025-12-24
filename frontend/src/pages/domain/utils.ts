const domainTypeToTitle = (domainType: string) => {
  switch (domainType) {
    case "proxyDeployment":
      return "Deployment";
    case "proxyStaticSite":
      return "Managed Domain";
    case "proxyUrl":
      return "Proxy URL";
    case "redirect":
      return "Redirect";
    default:
      return "Domain";
  }
};

export { domainTypeToTitle };
