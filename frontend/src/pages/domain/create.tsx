import { createSignal, Show } from "solid-js";
import { useNavigate } from "@solidjs/router";
import {
  Input,
  InputLabel,
  InputType,
  PageContainer,
  PageContainerBody,
  PageContainerHead,
  ButtonVariant,
  Button,
} from "~/components";
import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { doFetch } from "~/utils/do-fetch";
import {
  AddDomainToWorkspaceRequest,
  AddDomainToWorkspaceResponse,
} from "~/bindings";

// Domain extraction and validation logic
function extractDomain(input: string): {
  isUrl: boolean;
  domain: string;
  original: string;
} {
  let url = input.trim();

  // Add protocol if missing for URL parsing
  if (!url.match(/^https?:\/\//)) {
    url = "https://" + url;
  }

  try {
    const parsed = new URL(url);
    const hostname = parsed.hostname;

    // Check if it's a URL (has protocol, path, query, etc.)
    const isUrl =
      input.includes("://") ||
      input.includes("/") ||
      input.includes("?") ||
      input.includes("#");

    // Extract base domain (remove subdomains)
    const parts = hostname.split(".");
    const domain = parts.length > 2 ? parts.slice(-2).join(".") : hostname;

    return { isUrl, domain, original: input };
  } catch {
    return { isUrl: false, domain: input, original: input };
  }
}

const CreateDomainPage = () => {
  const [domainInput, setDomainInput] = createSignal("");
  const [error, setError] = createSignal("");
  const [suggestedDomain, setSuggestedDomain] = createSignal("");
  const [isSubmitting, setIsSubmitting] = createSignal(false);

  const [authState] = useAuthState();
  const [workspaceId] = useLastWorkspaceId();
  const navigate = useNavigate();

  const validateDomain = (input: string) => {
    if (!input.trim()) {
      setError("");
      setSuggestedDomain("");
      return;
    }

    const result = extractDomain(input);

    if (result.isUrl || result.domain !== result.original) {
      setError(
        "Please enter a valid domain (host + TLD only, no subdomains or URLs)"
      );
      setSuggestedDomain(result.domain);
    } else {
      setError("");
      setSuggestedDomain("");
    }
  };

  const handleInputChange = (value: string) => {
    setDomainInput(value);
    validateDomain(value);
  };

  const handleSuggestionClick = () => {
    const suggested = suggestedDomain();
    setDomainInput(suggested);
    setError("");
    setSuggestedDomain("");
  };

  const onSubmit = async (e: SubmitEvent) => {
    e.preventDefault();

    const auth = authState();
    const wsId = workspaceId();

    if (!auth || auth.type !== "LoggedIn" || !wsId) {
      console.error("User is not logged in or workspace ID missing");
      return;
    }

    const domain = domainInput().trim();

    if (!domain) {
      setError("Domain is required");
      return;
    }

    // Final validation before submit
    const result = extractDomain(domain);
    if (result.isUrl || result.domain !== result.original) {
      setError("Please use the suggested domain or enter a valid domain");
      setSuggestedDomain(result.domain);
      return;
    }

    setIsSubmitting(true);

    try {
      const requestBody: AddDomainToWorkspaceRequest = {
        domain: domain,
        nameserverType: "external",
      };

      const response = await doFetch<AddDomainToWorkspaceResponse>(
        `${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/domain`,
        {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${auth.accessToken}`,
          },
          body: JSON.stringify(requestBody),
        }
      );

      console.log("Domain added successfully:", response.data);
      navigate("/domains");
    } catch (error) {
      console.error("Error adding domain:", error);
      setError("Failed to add domain. Please try again.");
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <PageContainer>
      <PageContainerHead
        title="Add Domain"
        subTitle="Add a new domain to your workspace"
      />
      <PageContainerBody>
        <form onSubmit={onSubmit} class="space-y-6">
          <div class="bg-secondary-light p-6 rounded-lg border border-white/5">
            <div class="space-y-4">
              <div class="flex flex-col gap-2">
                <InputLabel
                  parentClass="flex-2"
                  for="domain-name"
                  label="Domain Name"
                />
                <Input
                  id="domain-name"
                  class="flex-10"
                  name="domain-name"
                  placeholder="example.com"
                  type={InputType.Text}
                  value={domainInput()}
                  onInput={(e) => handleInputChange(e.currentTarget.value)}
                  required
                />
                <Show when={error()}>
                  <p class="text-red-500 text-sm mt-1">{error()}</p>
                </Show>
                <Show when={suggestedDomain()}>
                  <p class="text-gray-400 text-sm mt-1">
                    Did you mean{" "}
                    <button
                      type="button"
                      onClick={handleSuggestionClick}
                      class="text-primary hover:underline font-medium"
                    >
                      {suggestedDomain()}
                    </button>
                    ?
                  </p>
                </Show>
              </div>

              <div class="bg-black/20 p-4 rounded border border-white/5">
                <h4 class="text-white text-sm font-semibold mb-2">
                  Domain Requirements:
                </h4>
                <ul class="text-gray-400 text-sm space-y-1 list-disc list-inside">
                  <li>Enter only the base domain (e.g., example.com)</li>
                  <li>Do not include subdomains (e.g., www.example.com)</li>
                  <li>Do not include protocols (e.g., https://)</li>
                  <li>Do not include paths or query parameters</li>
                  <li>We currently don't support non-ASCII domains</li>
                </ul>
              </div>
            </div>
          </div>

          <div class="w-full flex justify-end gap-4">
            <Button
              variant={ButtonVariant.Outlined}
              type="button"
              onClick={() => navigate("/domains")}
            >
              Cancel
            </Button>
            <Button
              variant={ButtonVariant.Contained}
              type="submit"
              disabled={isSubmitting() || !!error()}
            >
              {isSubmitting() ? "Adding..." : "Add Domain"}
            </Button>
          </div>
        </form>
      </PageContainerBody>
    </PageContainer>
  );
};

export default CreateDomainPage;
