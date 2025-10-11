// Validation result type
export interface ValidationResult {
    isValid: boolean;
    error?: string;
}

// Field validation rules
export interface ValidationRule {
    required?: boolean;
    minLength?: number;
    maxLength?: number;
    pattern?: RegExp;
    custom?: (value: string) => ValidationResult;
}

// Form validation utilities
export class ValidationUtil {
    // Email validation
    static validateEmail(email: string): ValidationResult {
        if (!email.trim()) {
            return { isValid: false, error: 'Email is required' };
        }

        const emailPattern = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
        if (!emailPattern.test(email)) {
            return { isValid: false, error: 'Please enter a valid email address' };
        }

        return { isValid: true };
    }

    // Password validation
    static validatePassword(password: string): ValidationResult {
        if (!password) {
            return { isValid: false, error: 'Password is required' };
        }

        if (password.length < 8) {
            return { isValid: false, error: 'Password must be at least 8 characters long' };
        }

        // Check for at least one uppercase, one lowercase, one number
        const hasUppercase = /[A-Z]/.test(password);
        const hasLowercase = /[a-z]/.test(password);
        const hasNumber = /\d/.test(password);

        if (!hasUppercase || !hasLowercase || !hasNumber) {
            return { 
                isValid: false, 
                error: 'Password must contain at least one uppercase letter, one lowercase letter, and one number' 
            };
        }

        return { isValid: true };
    }

    // Confirm password validation
    static validateConfirmPassword(password: string, confirmPassword: string): ValidationResult {
        if (!confirmPassword) {
            return { isValid: false, error: 'Please confirm your password' };
        }

        if (password !== confirmPassword) {
            return { isValid: false, error: 'Passwords do not match' };
        }

        return { isValid: true };
    }

    // Username validation
    static validateUsername(username: string): ValidationResult {
        if (!username.trim()) {
            return { isValid: false, error: 'Username is required' };
        }

        if (username.length < 3) {
            return { isValid: false, error: 'Username must be at least 3 characters long' };
        }

        if (username.length > 30) {
            return { isValid: false, error: 'Username must be less than 30 characters' };
        }

        // Username can only contain letters, numbers, underscores, and hyphens
        const usernamePattern = /^[a-zA-Z0-9_-]+$/;
        if (!usernamePattern.test(username)) {
            return { 
                isValid: false, 
                error: 'Username can only contain letters, numbers, underscores, and hyphens' 
            };
        }

        return { isValid: true };
    }

    // Required field validation
    static validateRequired(value: string, fieldName: string): ValidationResult {
        if (!value.trim()) {
            return { isValid: false, error: `${fieldName} is required` };
        }
        return { isValid: true };
    }

    // Generic field validation with rules
    static validateField(value: string, rules: ValidationRule, fieldName: string): ValidationResult {
        // Required validation
        if (rules.required && !value.trim()) {
            return { isValid: false, error: `${fieldName} is required` };
        }

        // Skip other validations if field is empty and not required
        if (!value.trim() && !rules.required) {
            return { isValid: true };
        }

        // Min length validation
        if (rules.minLength && value.length < rules.minLength) {
            return { 
                isValid: false, 
                error: `${fieldName} must be at least ${rules.minLength} characters long` 
            };
        }

        // Max length validation
        if (rules.maxLength && value.length > rules.maxLength) {
            return { 
                isValid: false, 
                error: `${fieldName} must be less than ${rules.maxLength} characters` 
            };
        }

        // Pattern validation
        if (rules.pattern && !rules.pattern.test(value)) {
            return { 
                isValid: false, 
                error: `${fieldName} format is invalid` 
            };
        }

        // Custom validation
        if (rules.custom) {
            return rules.custom(value);
        }

        return { isValid: true };
    }

    // OTP validation
    static validateOTP(otp: string): ValidationResult {
        if (!otp.trim()) {
            return { isValid: false, error: 'Verification code is required' };
        }

        if (otp.length !== 6) {
            return { isValid: false, error: 'Verification code must be 6 digits' };
        }

        if (!/^\d{6}$/.test(otp)) {
            return { isValid: false, error: 'Verification code must contain only numbers' };
        }

        return { isValid: true };
    }

    // Workspace name validation
    static validateWorkspaceName(name: string): ValidationResult {
        if (!name.trim()) {
            return { isValid: false, error: 'Workspace name is required' };
        }

        if (name.length < 2) {
            return { isValid: false, error: 'Workspace name must be at least 2 characters long' };
        }

        if (name.length > 50) {
            return { isValid: false, error: 'Workspace name must be less than 50 characters' };
        }

        // Workspace name can contain letters, numbers, spaces, hyphens, and underscores
        const namePattern = /^[a-zA-Z0-9\s_-]+$/;
        if (!namePattern.test(name)) {
            return { 
                isValid: false, 
                error: 'Workspace name can only contain letters, numbers, spaces, hyphens, and underscores' 
            };
        }

        return { isValid: true };
    }

    // Workspace description validation (optional field)
    static validateWorkspaceDescription(description: string): ValidationResult {
        if (description && description.length > 500) {
            return { 
                isValid: false, 
                error: 'Workspace description must be less than 500 characters' 
            };
        }

        return { isValid: true };
    }
}

// Form validation state management
export interface FormFieldState {
    value: string;
    error?: string;
    touched: boolean;
}

export interface FormState {
    [fieldName: string]: FormFieldState;
}

export class FormValidator {
    private state: FormState = {};
    private rules: { [fieldName: string]: ValidationRule } = {};

    constructor(initialState: { [fieldName: string]: string } = {}) {
        Object.keys(initialState).forEach(fieldName => {
            this.state[fieldName] = {
                value: initialState[fieldName],
                error: undefined,
                touched: false,
            };
        });
    }

    // Set validation rules for a field
    setFieldRules(fieldName: string, rules: ValidationRule): void {
        this.rules[fieldName] = rules;
    }

    // Update field value and validate
    updateField(fieldName: string, value: string): ValidationResult {
        if (!this.state[fieldName]) {
            this.state[fieldName] = { value: '', error: undefined, touched: false };
        }

        this.state[fieldName].value = value;
        this.state[fieldName].touched = true;

        const result = this.validateSingleField(fieldName);
        this.state[fieldName].error = result.error;

        return result;
    }

    // Validate a single field
    private validateSingleField(fieldName: string): ValidationResult {
        const fieldState = this.state[fieldName];
        const fieldRules = this.rules[fieldName];

        if (!fieldState || !fieldRules) {
            return { isValid: true };
        }

        return ValidationUtil.validateField(fieldState.value, fieldRules, fieldName);
    }

    // Validate all fields
    validateAll(): boolean {
        let isFormValid = true;

        Object.keys(this.state).forEach(fieldName => {
            const result = this.validateSingleField(fieldName);
            this.state[fieldName].error = result.error;
            this.state[fieldName].touched = true;

            if (!result.isValid) {
                isFormValid = false;
            }
        });

        return isFormValid;
    }

    // Get field state
    getFieldState(fieldName: string): FormFieldState | undefined {
        return this.state[fieldName];
    }

    // Get field value
    getFieldValue(fieldName: string): string {
        return this.state[fieldName]?.value || '';
    }

    // Get field error
    getFieldError(fieldName: string): string | undefined {
        return this.state[fieldName]?.error;
    }

    // Check if field has error
    hasFieldError(fieldName: string): boolean {
        const fieldState = this.state[fieldName];
        return fieldState?.touched && !!fieldState.error;
    }

    // Get all form values
    getFormValues(): { [fieldName: string]: string } {
        const values: { [fieldName: string]: string } = {};
        Object.keys(this.state).forEach(fieldName => {
            values[fieldName] = this.state[fieldName].value;
        });
        return values;
    }

    // Reset form
    reset(): void {
        Object.keys(this.state).forEach(fieldName => {
            this.state[fieldName] = {
                value: '',
                error: undefined,
                touched: false,
            };
        });
    }

    // Check if form is valid
    isValid(): boolean {
        return Object.values(this.state).every(fieldState => 
            !fieldState.touched || !fieldState.error
        );
    }
}

// Export validation utilities
export { ValidationUtil as Validation };
export default ValidationUtil;