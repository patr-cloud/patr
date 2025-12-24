const domainTypeToTitle = (domainType: string) => {
  switch (domainType) {
    case "proxyDeployment":
      return "Deployment";
    case "proxyStaticSite":
      return "Static Site";
    case "proxyUrl":
      return "Proxy URL";
    case "redirect":
      return "Redirect";
    default:
      return "Select A URL Type";
  }
};

export { domainTypeToTitle };
