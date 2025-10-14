import { JSX, mergeProps } from "solid-js";
import get from "~/utils/func";
import { MaybeAccessor } from "~/utils/types";

interface TableProps<TItem> {
  /** Flex Grid ratio */
  column_grids: string[];
  /** Additional Classes for the table.  */
  class?: MaybeAccessor<string>;
  /** Table Headings */
  headings: JSX.Element[];
  /** Table Rows */
  rows: MaybeAccessor<TItem[]>;
  /** Table Row Render Function */
  renderRow?: (item: TItem, index: number) => JSX.Element;
}

const Table = <TItem,>(rawProps: TableProps<TItem>) => {
  const props = mergeProps(
    {
      class: "",
    },
    rawProps
  );
  return (
    <table class={`rounded-xs w-full text-white ${get(props.class)}`}>
      <thead class="flex items-center justify-center py-sm bg-secondary-medium w-full rounded-t-xs">
        <tr class="flex items-center justify-center px-xl w-full">
          {props.headings.map((heading, index) => (
            <th
              class={`flex items-center justify-center text-sm font-medium ${
                props.column_grids.at(index) ?? ""
              }`}
            >
              {heading}
            </th>
          ))}
        </tr>
      </thead>

      <tbody class="w-full h-full flex flex-col justify-start items-start rounded-b-xs">
        {get(props.rows).length === 0 && (
          <tr class="w-full flex justify-center items-center p-md text-grey bg-secondary-light rounded-b-xs">
            <td>No data found.</td>
          </tr>
        )}
        {get(props.rows).map((row, index) => (
          <tr class="border-b border-grey last-of-type:rounded-b-xs">
            {props.renderRow ? props.renderRow(row, index) : null}
          </tr>
        ))}
      </tbody>
    </table>
  );
};

export default Table;
