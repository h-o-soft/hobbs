import { createSignal, createContext, useContext, createEffect, type ParentComponent } from 'solid-js';
import type { MeResponse } from '../types';
import * as authApi from '../api/auth';
import { getAccessToken, clearTokens } from '../api/client';

interface AuthState {
  user: MeResponse | null;
  isAuthenticated: boolean;
  isLoading: boolean;
}

interface AuthActions {
  login: (username: string, password: string) => Promise<void>;
  register: (username: string, password: string, nickname: string, email?: string) => Promise<void>;
  logout: () => Promise<void>;
  checkAuth: () => Promise<void>;
}

type AuthContextValue = [AuthState, AuthActions];

const AuthContext = createContext<AuthContextValue>();

export const AuthProvider: ParentComponent = (props) => {
  const [user, setUser] = createSignal<MeResponse | null>(null);
  const [isLoading, setIsLoading] = createSignal(true);

  const state = {
    get user() { return user(); },
    get isAuthenticated() { return user() !== null; },
    get isLoading() { return isLoading(); },
  };

  const actions: AuthActions = {
    async login(username: string, password: string) {
      await authApi.login({ username, password });
      const me = await authApi.getMe();
      setUser(me);
    },

    async register(username: string, password: string, nickname: string, email?: string) {
      await authApi.register({ username, password, nickname, email });
      const me = await authApi.getMe();
      setUser(me);
    },

    async logout() {
      try {
        await authApi.logout();
      } finally {
        setUser(null);
      }
    },

    async checkAuth() {
      const token = getAccessToken();
      if (!token) {
        setIsLoading(false);
        return;
      }

      try {
        const me = await authApi.getMe();
        setUser(me);
      } catch {
        clearTokens();
        setUser(null);
      } finally {
        setIsLoading(false);
      }
    },
  };

  // Check auth on mount
  createEffect(() => {
    actions.checkAuth();
  });

  return (
    <AuthContext.Provider value={[state, actions]}>
      {props.children}
    </AuthContext.Provider>
  );
};

export function useAuth(): AuthContextValue {
  const context = useContext(AuthContext);
  if (!context) {
    throw new Error('useAuth must be used within an AuthProvider');
  }
  return context;
}
