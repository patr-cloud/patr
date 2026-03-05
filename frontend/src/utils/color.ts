/*
 * Color enumeration for consistent color usage across the application.
 * Each color represents a specific theme or purpose.
 */
export enum Color {
	Primary = "primary",
	Secondary = "secondary",
	White = "white",
	Black = "black",
	Grey = "grey",
	Success = "success",
	/* Warning orange color. */
	Warning = "warning",
	Error = "error",
	Info = "info",
	Disabled = "disabled",
}

/**
 * Button variants for different styles of buttons.
 * @enum {typeof ButtonVariant[keyof typeof ButtonVariant]}
 */
const ButtonVariant = {
	/** An Outlined Link. This is a button without a background, but with an outline */
	Outlined: "outlined",
	/** A Plain Link. This is a button without a background or an outline. Looks like an anchor tag */
	Plain: "plain",
	/** A Contained Link. This is a button with a background and an outline */
	Contained: "contained",
};

export type ButtonVariantEnum = (typeof ButtonVariant)[keyof typeof ButtonVariant];

/**
 * CopyableField variants for different visual styles.
 * @enum {typeof CopyableFieldVariant[keyof typeof CopyableFieldVariant]}
 */
const CopyableFieldVariant = {
	/** Styled like a disabled Input (border, bg, rounded-xs). */
	Input: "input",
	/** Simple div with minimal styling for compact/inline contexts. */
	Text: "text",
} as const;

export type CopyableFieldVariantEnum = (typeof CopyableFieldVariant)[keyof typeof CopyableFieldVariant];

export { ButtonVariant, CopyableFieldVariant };
