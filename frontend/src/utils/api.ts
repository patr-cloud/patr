import { useAuthState, type AuthState } from "./state";

// API response format
export type ApiResponse<T> = 
    | ({ success: true } & T)
    | { success: false, error: string, message: string };

// Authentication types
export interface LoginCredentials {
    userId: string;      // Username or email
    password: string;
}

export interface SignupData {
    username: string;
    email: string;
    password: string;
    confirmPassword: string;
}

export interface AuthTokens {
    accessToken: string;
    refreshToken: string;
}

// Workspace types
export interface Workspace {
    id: string;
    name: string;
    description?: string;
    createdAt: string;
    updatedAt: string;
}

export interface WorkspaceData {
    name: string;
    description?: string;
}

// Deployment types
export interface Deployment {
    id: string;
    name: string;
    status: 'running' | 'stopped' | 'error' | 'pending' | 'deploying';
    image?: string;
    createdAt: string;
    updatedAt: string;
    lastDeployedAt?: string;
    url?: string;
}

// User info type
export interface UserInfo {
    id: string;
    email: string;
    username: string;
}

class ApiUtility {
    private baseUrl: string;
    private authState: () => AuthState;
    private setAuthState: (state: AuthState) => void;

    constructor() {
        this.baseUrl = '/api'; // Adjust based on your API base URL
        const [authState, setAuthState] = useAuthState();
        this.authState = authState;
        this.setAuthState = setAuthState;
    }

    private async makeRequest<T>(
        endpoint: string, 
        options: RequestInit = {}
    ): Promise<ApiResponse<T>> {
        const url = `${this.baseUrl}${endpoint}`;
        const authState = this.authState();
        
        // Add authentication headers if logged in
        const headers: Record<string, string> = {
            'Content-Type': 'application/json',
            ...(options.headers as Record<string, string> || {}),
        };

        if (authState.type === 'LoggedIn') {
            headers['Authorization'] = `Bearer ${authState.accessToken}`;
        }

        try {
            const response = await fetch(url, {
                ...options,
                headers,
            });

            // Handle 401 errors by attempting token refresh
            if (response.status === 401 && authState.type === 'LoggedIn') {
                const refreshResult = await this.refreshToken();
                if (refreshResult.success) {
                    // Retry the original request with new token
                    const newAuthState = this.authState();
                    if (newAuthState.type === 'LoggedIn') {
                        const retryHeaders: Record<string, string> = {
                            'Content-Type': 'application/json',
                            ...(options.headers as Record<string, string> || {}),
                            'Authorization': `Bearer ${newAuthState.accessToken}`,
                        };
                        const retryResponse = await fetch(url, {
                            ...options,
                            headers: retryHeaders,
                        });
                        return await this.parseResponse<T>(retryResponse);
                    }
                }
                // If refresh failed, clear auth state
                this.clearAuthTokens();
                return {
                    success: false,
                    error: 'authentication_failed',
                    message: 'Authentication failed. Please log in again.'
                };
            }

            return await this.parseResponse<T>(response);
        } catch (error) {
            return {
                success: false,
                error: 'network_error',
                message: error instanceof Error ? error.message : 'Network error occurred'
            };
        }
    }

    private async parseResponse<T>(response: Response): Promise<ApiResponse<T>> {
        try {
            const data = await response.json();
            
            if (response.ok) {
                return { success: true, ...data };
            } else {
                return {
                    success: false,
                    error: data.error || 'unknown_error',
                    message: data.message || 'An error occurred'
                };
            }
        } catch (error) {
            return {
                success: false,
                error: 'parse_error',
                message: 'Failed to parse response'
            };
        }
    }

    // Authentication endpoints
    async login(credentials: LoginCredentials): Promise<ApiResponse<AuthTokens>> {
        return this.makeRequest<AuthTokens>('/auth/login', {
            method: 'POST',
            body: JSON.stringify(credentials),
        });
    }

    async signup(userData: SignupData): Promise<ApiResponse<{ email: string }>> {
        return this.makeRequest<{ email: string }>('/auth/signup', {
            method: 'POST',
            body: JSON.stringify(userData),
        });
    }

    async confirmEmail(otp: string, email: string): Promise<ApiResponse<void>> {
        return this.makeRequest<void>('/auth/confirm-email', {
            method: 'POST',
            body: JSON.stringify({ otp, email }),
        });
    }

    async refreshToken(): Promise<ApiResponse<AuthTokens>> {
        const authState = this.authState();
        if (authState.type !== 'LoggedIn') {
            return {
                success: false,
                error: 'not_authenticated',
                message: 'Not authenticated'
            };
        }

        return this.makeRequest<AuthTokens>('/auth/refresh', {
            method: 'POST',
            body: JSON.stringify({ refreshToken: authState.refreshToken }),
        });
    }

    // Workspace endpoints
    async createWorkspace(data: WorkspaceData): Promise<ApiResponse<Workspace>> {
        return this.makeRequest<Workspace>('/workspaces', {
            method: 'POST',
            body: JSON.stringify(data),
        });
    }

    async getWorkspaces(): Promise<ApiResponse<{ workspaces: Workspace[] }>> {
        return this.makeRequest<{ workspaces: Workspace[] }>('/workspaces');
    }

    // Deployment endpoints
    async getDeployments(workspaceId: string): Promise<ApiResponse<{ deployments: Deployment[] }>> {
        return this.makeRequest<{ deployments: Deployment[] }>(`/workspaces/${workspaceId}/deployments`);
    }

    // Utility methods
    setAuthTokens(tokens: AuthTokens): void {
        this.setAuthState({
            type: 'LoggedIn',
            accessToken: tokens.accessToken,
            refreshToken: tokens.refreshToken,
        });
    }

    clearAuthTokens(): void {
        this.setAuthState({ type: 'LoggedOut' });
    }

    isAuthenticated(): boolean {
        return this.authState().type === 'LoggedIn';
    }
}

// Create singleton instance
export const api = new ApiUtility();