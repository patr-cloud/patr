import { FiPlus } from "solid-icons/fi";
import { createMemo, createSignal, mergeProps, Suspense } from "solid-js";
import { Workspace, type WithId } from "~/bindings";
import {
  Button,
  ButtonVariant,
  InputDropdown,
  InputDropdownCheckBox,
} from "~/components";
import useFetchPermissions from "~/hooks/use-fetch/use-fetch-permissions";
import { get, parseCamelCase, parsePermissionName } from "~/utils/func";
import { MaybeAccessor } from "~/utils/types";

interface WorkspaceRolesProps {
  /** The Workspace Info */
  workspace: WithId<Workspace>;
  /** Additional Classes to apply */
  class?: MaybeAccessor<string>;
}

const WorkspaceRoles = (rawProps: WorkspaceRolesProps) => {
  const props = mergeProps({ class: "" }, rawProps);
  const [selectedPermissionIds, setSelectedPermissionIds] = createSignal<
    Set<string>
  >(new Set());
  const [selectedResourceType, setSelectedResourceType] =
    createSignal<string>("");

  const [permissions] = useFetchPermissions(props.workspace.id);
  const [includeExcludeMode, setIncludeExcludeMode] = createSignal<
    "all" | "include" | "exclude"
  >("all");

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

  return (
    <>
      <Suspense
        fallback={
          <div class="text-gray-400 text-sm">Loading permissions...</div>
        }
      >
        <div class={`flex gap-3 ${get(props.class)}`}>
          {/* column 1: resource types */}
          <div class="flex flex-[2.5] flex-col gap-3">
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
          <div class="flex flex-[2.5] flex-col gap-3">
            <InputDropdownCheckBox
              onToggle={(val) => togglePermissionId(val)}
              checked={() => Array.from(selectedPermissionIds())}
              placeholder={() =>
                Array.from(selectedPermissionIds())
                  .map((s) => permissionActions().find((p) => p.id === s)?.name)
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

          {/* Column 3: Include/Exclude */}
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

          {/* Column 4: Add Button */}

          {/* <div class="flex items-center justify-center flex-[2.5]">
            
          </div> */}

          <div class="flex items-end justify-center flex-[0.5]">
            <Button type="button" variant={ButtonVariant.Contained}>
              <FiPlus />
            </Button>
          </div>
        </div>
      </Suspense>
    </>
  );
};

export default WorkspaceRoles;
