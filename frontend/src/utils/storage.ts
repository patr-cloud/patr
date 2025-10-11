// User information type (stored in session storage)
export interface UserInfo {
    id: string;
    email: string;
    username: string;
}

// Workspace state type (stored in local storage)
export interface WorkspaceState {
    currentWorkspaceId?: string;
    workspaceIds: string[];
}

// Session Storage utilities
export class SessionStorageUtil {
    private static readonly USER_INFO_KEY = 'userInfo';

    static setUserInfo(userInfo: UserInfo): void {
        try {
            sessionStorage.setItem(this.USER_INFO_KEY, JSON.stringify(userInfo));
        } catch (error) {
            console.warn('Failed to save user info to session storage:', error);
        }
    }

    static getUserInfo(): UserInfo | null {
        try {
            const stored = sessionStorage.getItem(this.USER_INFO_KEY);
            return stored ? JSON.parse(stored) : null;
        } catch (error) {
            console.warn('Failed to retrieve user info from session storage:', error);
            return null;
        }
    }

    static clearUserInfo(): void {
        try {
            sessionStorage.removeItem(this.USER_INFO_KEY);
        } catch (error) {
            console.warn('Failed to clear user info from session storage:', error);
        }
    }

    static hasUserInfo(): boolean {
        return this.getUserInfo() !== null;
    }
}

// Local Storage utilities
export class LocalStorageUtil {
    private static readonly WORKSPACE_STATE_KEY = 'workspaceState';

    static setWorkspaceState(workspaceState: WorkspaceState): void {
        try {
            localStorage.setItem(this.WORKSPACE_STATE_KEY, JSON.stringify(workspaceState));
        } catch (error) {
            console.warn('Failed to save workspace state to local storage:', error);
        }
    }

    static getWorkspaceState(): WorkspaceState {
        try {
            const stored = localStorage.getItem(this.WORKSPACE_STATE_KEY);
            return stored ? JSON.parse(stored) : { workspaceIds: [] };
        } catch (error) {
            console.warn('Failed to retrieve workspace state from local storage:', error);
            return { workspaceIds: [] };
        }
    }

    static setCurrentWorkspace(workspaceId: string): void {
        const currentState = this.getWorkspaceState();
        const updatedState: WorkspaceState = {
            ...currentState,
            currentWorkspaceId: workspaceId,
        };
        
        // Add to workspace IDs if not already present
        if (!currentState.workspaceIds.includes(workspaceId)) {
            updatedState.workspaceIds = [...currentState.workspaceIds, workspaceId];
        }
        
        this.setWorkspaceState(updatedState);
    }

    static getCurrentWorkspaceId(): string | undefined {
        return this.getWorkspaceState().currentWorkspaceId;
    }

    static getWorkspaceIds(): string[] {
        return this.getWorkspaceState().workspaceIds;
    }

    static addWorkspaceId(workspaceId: string): void {
        const currentState = this.getWorkspaceState();
        if (!currentState.workspaceIds.includes(workspaceId)) {
            const updatedState: WorkspaceState = {
                ...currentState,
                workspaceIds: [...currentState.workspaceIds, workspaceId],
            };
            this.setWorkspaceState(updatedState);
        }
    }

    static removeWorkspaceId(workspaceId: string): void {
        const currentState = this.getWorkspaceState();
        const updatedState: WorkspaceState = {
            ...currentState,
            workspaceIds: currentState.workspaceIds.filter(id => id !== workspaceId),
        };
        
        // Clear current workspace if it's the one being removed
        if (currentState.currentWorkspaceId === workspaceId) {
            updatedState.currentWorkspaceId = undefined;
        }
        
        this.setWorkspaceState(updatedState);
    }

    static clearWorkspaceState(): void {
        try {
            localStorage.removeItem(this.WORKSPACE_STATE_KEY);
        } catch (error) {
            console.warn('Failed to clear workspace state from local storage:', error);
        }
    }

    static hasWorkspaces(): boolean {
        return this.getWorkspaceIds().length > 0;
    }
}

// Combined storage utilities for common operations
export class StorageUtil {
    static clearAllUserData(): void {
        SessionStorageUtil.clearUserInfo();
        LocalStorageUtil.clearWorkspaceState();
    }

    static isUserDataAvailable(): boolean {
        return SessionStorageUtil.hasUserInfo() && LocalStorageUtil.hasWorkspaces();
    }
}

// Export individual utilities for convenience
export { SessionStorageUtil as SessionStorage };
export { LocalStorageUtil as LocalStorage };
export { StorageUtil as Storage };