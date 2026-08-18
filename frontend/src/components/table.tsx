import { For, JSX, mergeProps } from "solid-js";
import { get } from "~/utils/func";
import { MaybeAccessor } from "~/utils/types";

interface TableProps<TItem> {
	/** Flex Grid ratio */
	column_grids: string[];
	/** Additional Classes for the table.  */
	class?: MaybeAccessor<string>;
	/** Table Headings */
	headings: JSX.Element[];
	/** Horizontal alignment of the headings. Defaults to `center`; use `left` when the
	 * table's cell contents are themselves left-aligned, so the two line up. */
	heading_align?: "left" | "center";
	/** Table Rows */
	rows: MaybeAccessor<TItem[]>;
	/** Table Row Render Function */
	renderRow?: (item: TItem, index: number) => JSX.Element;
}

const TableRow = <TItem extends Record<string, unknown>>(props: {
	item: TItem;
	class?: MaybeAccessor<string>;
	column_classes: string[];
}) => {
	return (
		<tr
			role="row"
			class={`border border-border-color min-h-10 cursor-pointer flex items-center justify-center w-full px-xl
        bg-secondary-light last-of-type:rounded-b-xs ${get(props.class)}`}
		>
			<For each={Object.values(props.item)}>
				{(row, index) => (
					<td
						role="cell"
						class={`flex items-center justify-center ${props.column_classes.at(index()) ?? ""}`}
					>
						{row as string}
					</td>
				)}
			</For>
		</tr>
	);
};

const Table = <TItem extends Record<string, unknown>>(rawProps: TableProps<TItem>) => {
	const props = mergeProps(
		{
			class: "",
			heading_align: "center" as const,
		},
		rawProps
	);
	return (
		<div class="w-full overflow-x-auto">
			<table role="table" class={`rounded-xs w-full min-w-150 text-white ${get(props.class)}`}>
				<thead class="flex items-center justify-center py-sm bg-secondary-medium w-full rounded-t-xs">
					<tr role="row" class="flex items-center justify-center px-md md:px-xl w-full">
						<For each={props.headings}>
							{(heading, index) => (
								<th
									role="columnheader"
									class={`flex items-center ${
										props.heading_align === "left" ? "justify-start" : "justify-center"
									} text-sm font-medium ${props.column_grids.at(index()) ?? ""}`}
								>
									{heading}
								</th>
							)}
						</For>
					</tr>
				</thead>

				<tbody class="w-full h-full flex flex-col justify-start items-start rounded-b-xs">
					{get(props.rows).length === 0 && (
						<tr
							role="row"
							class="w-full flex justify-center items-center p-md text-grey bg-secondary-light rounded-b-xs"
						>
							<td role="cell">No data found.</td>
						</tr>
					)}
					<For each={get(props.rows)}>
						{(row, index) => <>{props.renderRow ? props.renderRow(row, index()) : null}</>}
					</For>
				</tbody>
			</table>
		</div>
	);
};

export { TableRow };
export default Table;
