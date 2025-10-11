# Design Document

## Overview

This design outlines the implementation of core authentication and workspace management pages for the Patr platform frontend. The solution builds upon the existing SolidJS architecture, design system, and component library to create a cohesive user experience that maintains the established space-themed aesthetic while providing robust functionality for user onboarding and deployment management.

## Architecture

### Frontend Framework
- **SolidJS**: Continue using the existing SolidJS framework with TypeScript
- **Router**: Leverage `@solidjs/router` for client-side navigation
- **State Management**: Extend the existing cookie-based authentication state management
- **Styling**: Build upon the existing Tailwind CSS configuration with custom design tokens

### Project Structure
```
frontend/src/
├── components/           # Reusable UI components
│   ├── button.tsx       # Existing button component
│   ├── input.tsx        # Existing input component
│   ├── loading-spinner.tsx    # New loading component
│   ├── error-message.tsx      # New error display component
│   ├── success-message.tsx    # New success display component
│   └── otp-input.tsx          # New OTP input component
├── pages/                # Layout components
│   ├── auth-layout.tsx   # Layout for authentication pages
│   └── app-layout.tsx    # Layout for authenticated pages
├── routes/
│   ├── logged-out-routes/
│   │   ├── login.tsx           # Enhanced existing login
│   │   ├── sign-up.tsx         # Enhanced existing sign-up
│   │   ├── confirm-signup.tsx  # New OTP confirmation
│   │   └── index.tsx
│   ├── logged-in-routes/
│   │   ├── create-workspace.tsx # New workspace creation
│   │   ├── deployments.tsx      # New deployment list
│   │   └── index.tsx
│   └── index.tsx
├── utils/
│   ├── api.ts           # New centralized API utility
│   ├── state.ts         # Enhanced state management
│   ├── storage.ts       # Local/session storage utilities
│   └── validation.ts    # New form validation utilities
└── types/
    └── api.ts           # API response type definitions
```

## Components and Interfaces

### Enhanced Authentication State Management

```typescript
// Simplified AuthState type (stored in cookies)
export type AuthState = {
    type: "LoggedIn",
    accessToken: string,
    refreshToken: string,
} | {
    type: "LoggedOut",
};

// User information (stored in session storage)
export type UserInfo = {
    id: string,
    email: string,
    username: string,
};

// Workspace state (stored in local storage)
export type WorkspaceState = {
    currentWorkspaceId?: string,
    workspaceIds: string[],
};
```

### API Utility Design

```typescript
// API response format
type ApiResponse<T> = 
    | { success: true } & T
    | { success: false, error: string, message: string };

interface ApiUtility {
    // Authentication endpoints
    login(credentials: LoginCredentials): Promise<ApiResponse<AuthTokens>>,
    signup(userData: SignupData): Promise<ApiResponse<{ email: string }>>,
    confirmEmail(otp: string, email: string): Promise<ApiResponse<void>>,
    
    // Workspace endpoints
    createWorkspace(data: WorkspaceData): Promise<ApiResponse<Workspace>>,
    getWorkspaces(): Promise<ApiResponse<{ workspaces: Workspace[] }>>,
    
    // Deployment endpoints
    getDeployments(workspaceId: string): Promise<ApiResponse<{ deployments: Deployment[] }>>,
    
    // Utility methods
    refreshToken(): Promise<ApiResponse<AuthTokens>>,
    setAuthTokens(tokens: AuthTokens): void,
    clearAuthTokens(): void,
}
```

### New UI Components

#### LoadingSpinner Component
- Uses design system colors (primary orange)
- Consistent sizing and animation
- Supports different sizes (sm, md, lg)

#### ErrorMessage Component
- Uses established error color (#d62b36)
- Supports dismissible and persistent variants
- Includes retry functionality where applicable

#### SuccessMessage Component
- Uses success color (#47c96c)
- Auto-dismiss after timeout
- Consistent styling with error messages

#### OTPInput Component
- Specialized input for 6-digit OTP codes
- Auto-focus and auto-advance between digits
- Paste support for complete codes
- Validation and error states

### Page Layouts

#### AuthLayout Component (pages/auth-layout.tsx)
- Shared layout for all authentication pages
- Maintains space theme background with stars, astronaut, planet, spaceship
- Responsive design for mobile and desktop
- Consistent footer with copyright

#### AppLayout Component (pages/app-layout.tsx)
- Layout for authenticated pages
- Header with Patr logo and user menu
- Navigation sidebar (for future expansion)
- Main content area with consistent padding

## Data Models

### Authentication Models
```typescript
interface LoginCredentials {
    userId: string,      // Username or email
    password: string,
}

interface SignupData {
    username: string,
    email: string,
    password: string,
    confirmPassword: string,
}

interface AuthTokens {
    accessToken: string,
    refreshToken: string,
}
```

### Workspace Models
```typescript
interface Workspace {
    id: string,
    name: string,
    description?: string,
    createdAt: string,
    updatedAt: string,
}

interface WorkspaceData {
    name: string,
    description?: string,
}
```

### Deployment Models
```typescript
interface Deployment {
    id: string,
    name: string,
    status: 'running' | 'stopped' | 'error' | 'pending' | 'deploying',
    image?: string,
    createdAt: string,
    updatedAt: string,
    lastDeployedAt?: string,
    url?: string,
}
```

## Error Handling

### Form Validation Strategy
- Client-side validation using custom validation utilities
- Real-time validation feedback
- Server-side validation error display
- Field-specific error messages

### API Error Handling
- Standardized error response format
- Automatic token refresh on 401 errors
- Network error detection and retry mechanisms
- User-friendly error message translation

### Error Display Patterns
- Inline field errors for form validation
- Toast notifications for operation results
- Page-level error states for critical failures
- Loading states during API operations

## Testing Strategy

### Component Testing
- Unit tests for individual components using Vitest
- Props validation and rendering tests
- User interaction testing with solid-testing-library
- Accessibility testing for form elements

### Integration Testing
- API utility function testing with mock responses
- Authentication flow testing
- Form submission and validation testing
- Navigation and routing testing

### End-to-End Testing
- Complete user registration and login flow
- Workspace creation and navigation
- Error handling and recovery scenarios
- Responsive design validation

## Implementation Phases

### Phase 1: Core Infrastructure
1. Set up API utility with authentication handling
2. Enhance authentication state management
3. Create base layout components
4. Implement loading and error UI components

### Phase 2: Authentication Pages
1. Update login page with API integration
2. Enhance sign-up page with validation
3. Create OTP confirmation page
4. Implement proper error handling and loading states

### Phase 3: Workspace Management
1. Create workspace creation page
2. Implement workspace state management
3. Add navigation between authentication and workspace flows
4. Handle edge cases and error scenarios

### Phase 4: Deployment Management
1. Create deployment list page
2. Implement deployment data fetching
3. Add empty states and loading indicators
4. Prepare foundation for future deployment management features

## Security Considerations

### Token Management
- Secure cookie storage for authentication tokens
- Automatic token refresh before expiration
- Proper token cleanup on logout
- CSRF protection considerations

### Form Security
- Input sanitization and validation
- Password strength requirements
- Rate limiting considerations for API calls
- Secure handling of sensitive data

### API Security
- Proper error message handling (avoid information leakage)
- Request timeout handling
- Secure communication protocols
- Authentication header management
##
 Storage Strategy

### Cookie Storage (Authentication)
- `authState`: Contains only authentication tokens and login status
- Secure, HttpOnly when possible
- Automatic expiration handling

### Session Storage (User Data)
- `userInfo`: User profile information (id, email, username)
- Cleared on browser tab close
- Fetched fresh on each session

### Local Storage (Workspace Data)
- `workspaceState`: Current workspace ID and list of workspace IDs
- Persists across browser sessions
- Lightweight data only (IDs, not full objects)

## Route Structure

### Parent Route Definitions
```typescript
// routes/logged-out-routes/index.tsx
export default function LoggedOutRoutes() {
  return (
    <>
      <Route path="/" component={() => <Navigate href="/login" />} />
      <Route path="/login" component={Login} />
      <Route path="/sign-up" component={SignUp} />
      <Route path="/confirm-signup" component={ConfirmSignup} />
    </>
  );
}

// routes/logged-in-routes/index.tsx  
export default function LoggedInRoutes() {
  return (
    <>
      <Route path="/" component={() => <Navigate href="/create-workspace" />} />
      <Route path="/create-workspace" component={CreateWorkspace} />
      <Route path="/deployments" component={Deployments} />
    </>
  );
}
```