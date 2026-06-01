import {
	For,
	JSX,
	ParentProps,
	createContext,
	mergeProps,
	useContext,
} from "solid-js";
import { get } from "~/utils/func";
import { MaybeAccessor } from "~/utils/types";

type CellAlign = "start" | "center" | "end";

interface TableContextValue {
	column_grids: string[];
}

const TableContext = createContext<TableContextValue>();

const useTableContext = (): TableContextValue => {
	const ctx = useContext(TableContext);
	if (!ctx) {
		return { column_grids: [] };
	}
	return ctx;
};

interface TableProps<TItem> {
	/** Flex Grid ratio — applied to both header columns and `<TableCell>` cells (matched by index). */
	column_grids: string[];
	/** Additional Classes for the table.  */
	class?: MaybeAccessor<string>;
	/** Table Headings */
	headings: JSX.Element[];
	/** Table Rows */
	rows: MaybeAccessor<TItem[]>;
	/** Table Row Render Function */
	renderRow?: (item: TItem, index: number) => JSX.Element;
	/** Optional text shown when there are no rows. */
	emptyMessage?: MaybeAccessor<string>;
}

interface TableRowProps {
	/** When provided, the row becomes clickable: keyboard focusable, activatable via Enter/Space. */
	onClick?: (e: MouseEvent | KeyboardEvent) => void;
	/** Accessible label for clickable rows (e.g. "Open deployment foo"). */
	"aria-label"?: string;
	class?: MaybeAccessor<string>;
}

interface TableCellProps {
	/** Index into the parent Table's `column_grids` — determines this cell's flex sizing. */
	index: number;
	/** Horizontal content alignment within the cell. Defaults to "start". */
	align?: CellAlign;
	class?: MaybeAccessor<string>;
}

const alignClass = (align: CellAlign): string => {
	switch (align) {
		case "center":
			return "justify-center";
		case "end":
			return "justify-end";
		case "start":
		default:
			return "justify-start";
	}
};

const TableRow = (rawProps: ParentProps<TableRowProps>) => {
	const props = mergeProps({ class: "" }, rawProps);

	const clickable = (): boolean => typeof rawProps.onClick === "function";

	const handleKeyDown = (e: KeyboardEvent) => {
		if (!clickable()) return;
		if (e.key === "Enter" || e.key === " ") {
			e.preventDefault();
			rawProps.onClick?.(e);
		}
	};

	return (
		<tr
			role="row"
			tabIndex={clickable() ? 0 : undefined}
			aria-label={rawProps["aria-label"]}
			onClick={clickable() ? rawProps.onClick : undefined}
			onKeyDown={clickable() ? handleKeyDown : undefined}
			class={`table-row ${
				clickable()
					? "cursor-pointer focus-visible:outline-2 focus-visible:outline-primary focus-visible:-outline-offset-2"
					: ""
			} ${get(props.class)}`}
		>
			{props.children}
		</tr>
	);
};

const TableCell = (rawProps: ParentProps<TableCellProps>) => {
	const props = mergeProps({ class: "", align: "start" as CellAlign }, rawProps);
	const ctx = useTableContext();
	const widthClass = (): string => ctx.column_grids.at(props.index) ?? "";

	return (
		<td
			role="cell"
			class={`flex items-center min-w-0 ${widthClass()} ${alignClass(
				props.align
			)} ${get(props.class)}`}
		>
			{props.children}
		</td>
	);
};

const Table = <TItem extends Record<string, unknown>>(rawProps: TableProps<TItem>) => {
	const props = mergeProps(
		{
			class: "",
			emptyMessage: "No data found.",
		},
		rawProps
	);

	const ctxValue: TableContextValue = {
		get column_grids() {
			return props.column_grids;
		},
	};

	return (
		<TableContext.Provider value={ctxValue}>
			<div class="w-full overflow-x-auto">
				<table
					role="table"
					class={`rounded-xs w-full min-w-150 text-white ${get(props.class)}`}
				>
					<thead class="flex items-center justify-center py-sm bg-secondary-medium w-full rounded-t-xs">
						<tr role="row" class="flex items-center justify-center px-md md:px-xl w-full">
							<For each={props.headings}>
								{(heading, index) => (
									<th
										role="columnheader"
										class={`flex items-center justify-center text-sm font-medium ${
											props.column_grids.at(index()) ?? ""
										}`}
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
								<td role="cell">{get(props.emptyMessage)}</td>
							</tr>
						)}
						<For each={get(props.rows)}>
							{(row, index) => (
								<>{props.renderRow ? props.renderRow(row, index()) : null}</>
							)}
						</For>
					</tbody>
				</table>
			</div>
		</TableContext.Provider>
	);
};

export { TableRow, TableCell };
export default Table;
