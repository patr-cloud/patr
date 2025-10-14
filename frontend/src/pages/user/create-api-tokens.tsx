import {
  Button,
  ButtonVariant,
  Input,
  InputLabel,
  InputType,
  PageContainer,
  PageContainerBody,
  PageContainerHead,
} from "~/components";

const CreateApiTokens = () => {
  return (
    <PageContainer>
      <PageContainerHead title="Create API Tokens" subTitle="subtitle" />
      <PageContainerBody class="flex flex-col justify-between gap-8">
        <div class="flex flex-col gap-6 items-start w-full">
          <h1 class="text-md">Create API Tokens</h1>

          <div class="flex gap-8 items-center w-full">
            <InputLabel
              parentClass="flex-2"
              for="token-name"
              label="Token Name"
            />
            <Input
              class="flex-10"
              name="token-name"
              placeholder="Enter Token Name"
              type={InputType.Text}
            />
          </div>

          <div class="flex gap-8 items-center w-full">
            <InputLabel
              parentClass="flex-2"
              for="allowed-ips"
              label="Allowed IP(s)"
              comments="By default, all IP addresses will be allowed. Enter Comma Separated Values."
            />
            <Input
              class="flex-10"
              name="token-name"
              placeholder="Enter Comma Seperated IP(s)"
              type={InputType.Text}
            />
          </div>

          <div class="flex gap-8 items-center w-full">
            <InputLabel
              parentClass="flex-2"
              label="Token Validity"
              comments="By default, the token will be valid forever from the date created."
            />

            <div class="flex items-center flex-10 gap-4">
              <InputLabel
                parentClass="flex-2"
                for="token-validity-from"
                label="Valid From"
              />
              <Input
                class="flex-10"
                name="token-validity"
                placeholder="Enter Token Validity in days"
                type={InputType.Date}
              />

              <InputLabel
                parentClass="flex-2 items-center"
                for="token-validity-to"
                label="to"
              />
              <Input
                class="flex-10"
                name="token-validity"
                placeholder="Enter Token Validity in days"
                type={InputType.Date}
              />
            </div>
          </div>
        </div>

        <div class="flex justify-end">
          <Button variant={ButtonVariant.Contained}>Create Token</Button>
        </div>
      </PageContainerBody>
    </PageContainer>
  );
};
export default CreateApiTokens;
