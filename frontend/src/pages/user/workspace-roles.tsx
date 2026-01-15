import { FiPlus } from "solid-icons/fi";
import { createMemo, createSignal, mergeProps, Show, Suspense } from "solid-js";
import { Workspace, WorkspacePermission, type WithId } from "~/bindings";
import {
  Button,
  ButtonVariant,
  InputDropdown,
  InputDropdownCheckBox,
  ListResources,
} from "~/components";
import useFetchPermissions from "~/hooks/use-fetch/use-fetch-permissions";
import {
  get,
  getResourceEndpoint,
  parseCamelCase,
  parsePermissionName,
} from "~/utils/func";
import { MaybeAccessor } from "~/utils/types";

interface WorkspaceRolesProps {
  /** The Workspace Info */
  workspace: MaybeAccessor<string>;
  /** Add Permission */
  addPermission: (permission: WorkspacePermission) => void;
  /** Additional Classes to apply */
  class?: MaybeAccessor<string>;
  /** Existing Permission */
  existingPermission?: MaybeAccessor<WorkspacePermission>;
}

const WorkspaceRoles = (rawProps: WorkspaceRolesProps) => {
  const props = mergeProps({ class: "" }, rawProps);
  const [selectedPermissionIds, setSelectedPermissionIds] = createSignal<
    Set<string>
  >(new Set());
  const [selectedResourceType, setSelectedResourceType] =
    createSignal<string>("");

  const [selectedResources, setSelectedResources] = createSignal<Set<string>>(
    new Set()
  );

  const [permissions] = useFetchPermissions(get(props.workspace));
  const [includeExcludeMode, setIncludeExcludeMode] = createSignal<
    "all" | "include" | "exclude"
  >("all");

  const toggleResource = (resourceId: string) => {
    console.log("Toggling resource:", resourceId);
    const newSet = new Set(selectedResources());
    if (newSet.has(resourceId)) {
      newSet.delete(resourceId);
    } else {
      newSet.add(resourceId);
    }
    setSelectedResources(newSet);
  };

  const permissionActions = createMemo(() => {
    return (permissions()?.permissions || []).filter((p) => {
      const parsed = parsePermissionName(p.name);
      return parsed.resourceType === selectedResourceType();
    });
  });

  const togglePermissionId = (permissionId: string) => {
    const newSet = new Set(selectedPermissionIds());
    if (newSet.has(permissionId)) {
      newSet.delete(permissionId);
    } else {
      newSet.add(permissionId);
    }
    setSelectedPermissionIds(newSet);
  };

  const onClickAdd = () => {
    console.log("Adding role with details:");
    console.log("Resource Type:", selectedResourceType());
    console.log("Permission IDs:", Array.from(selectedPermissionIds()));
    console.log("Include/Exclude Mode:", includeExcludeMode());
    console.log("Selected Resources:", Array.from(selectedResources()));

    const workspacePerm = Object.fromEntries(
      [...selectedPermissionIds()].map((permId) => {
        const includeMode = includeExcludeMode();
        let permissionObj;
        switch (includeMode) {
          case "all":
            permissionObj = {
              permissionType: "exclude" as const,
              resources: [],
            };
            break;
          case "include":
            permissionObj = {
              permissionType: "include" as const,
              resources: Array.from(selectedResources()),
            };
            break;
          case "exclude":
            permissionObj = {
              permissionType: "exclude" as const,
              resources: Array.from(selectedResources()),
            };
            break;
        }
        return [permId, permissionObj];
      })
    );

    const workspacePermObj = {
      type: "member" as const,
      ...workspacePerm,
    };
    console.log("Constructed Workspace Permission Object:", workspacePermObj);
    props.addPermission(workspacePermObj as WorkspacePermission);
  };

  return (
    <>
      <Suspense
        fallback={
          <div class="text-gray-400 text-sm">Loading permissions...</div>
        }
      >
        <div class={`flex gap-3 items-center ${get(props.class)}`}>
          {/* column 1: resource types */}
          <div class={`flex flex-[2.5] flex-col gap-3`}>
            <InputDropdown
              onSelect={(val) => {
                console.log(val);
                setSelectedResourceType(val);
              }}
              placeholder="Select Resource Type"
              value={selectedResourceType}
              options={Array.from(
                new Set(
                  (permissions()?.permissions || [])
                    .map((p) => parsePermissionName(p.name).resourceType)
                    .filter((r) => r)
                )
              ).map((resourceType) => ({
                label: parseCamelCase(resourceType),
                value: resourceType,
              }))}
            />
          </div>

          {/* Column 2: Permission Actions (filtered by selected resource type) */}
          <Show when={selectedResourceType() && permissionActions().length > 0}>
            <div class="flex flex-[2.5] flex-col gap-3">
              <InputDropdownCheckBox
                onToggle={(val) => togglePermissionId(val)}
                checked={() => Array.from(selectedPermissionIds())}
                placeholder={() =>
                  Array.from(selectedPermissionIds())
                    .map(
                      (s) => permissionActions().find((p) => p.id === s)?.name
                    )
                    .map((val) =>
                      val
                        ? parseCamelCase(parsePermissionName(val).action)
                        : undefined
                    )
                    .join(", ") || "Select Permissions"
                }
                options={() =>
                  permissionActions().map((p) => {
                    const parsed = parsePermissionName(p.name);
                    return {
                      label: `${parseCamelCase(parsed.action)}`,
                      value: p.id,
                    };
                  })
                }
              />
            </div>
          </Show>

          {/* Column 3: Include/Exclude */}
          <Show
            when={
              selectedResourceType() &&
              getResourceEndpoint(selectedResourceType())
            }
          >
            <div class="flex flex-[2.5] flex-col gap-3">
              <InputDropdown
                onSelect={(val) =>
                  setIncludeExcludeMode(val as "all" | "include" | "exclude")
                }
                placeholder="Select Include/Exclude Mode"
                value={includeExcludeMode}
                options={[
                  {
                    label: selectedResourceType()
                      ? `All ${parseCamelCase(selectedResourceType())}(s)`
                      : "All Resources",
                    value: "all",
                  },
                  {
                    label: selectedResourceType()
                      ? `Include Specific ${parseCamelCase(
                          selectedResourceType()
                        )}(s)`
                      : "Include Specific Resources",
                    value: "include",
                  },
                  {
                    label: selectedResourceType()
                      ? `Exclude Specific ${parseCamelCase(
                          selectedResourceType()
                        )}(s)`
                      : "Exclude Specific Resources",
                    value: "exclude",
                  },
                ]}
              />
            </div>
          </Show>

          {/* Column 4: Add Button */}
          <Show
            when={
              selectedResourceType() &&
              includeExcludeMode() !== "all" &&
              getResourceEndpoint(selectedResourceType())
            }
          >
            <div class="flex flex-col gap-[2.5]">
              <ListResources
                workspaceId={props.workspace}
                resourceType={selectedResourceType}
                selectedResources={selectedResources}
                toggleResource={toggleResource}
              />
            </div>
          </Show>

          <div class="flex items-end justify-center">
            <Button
              onClick={onClickAdd}
              type="button"
              variant={ButtonVariant.Contained}
            >
              <FiPlus />
            </Button>
          </div>
        </div>
      </Suspense>
    </>
  );
};

export default WorkspaceRoles;
