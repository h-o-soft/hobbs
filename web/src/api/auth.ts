import { api, setTokens, clearTokens } from './client';
import type {
  LoginRequest,
  LoginResponse,
  RegisterRequest,
  MeResponse,
} from '../types';

export async function login(credentials: LoginRequest): Promise<LoginResponse> {
  const response = await api.post<LoginResponse>('/auth/login', credentials, { skipAuth: true });
  setTokens(response.access_token, response.refresh_token);
  return response;
}

export async function register(data: RegisterRequest): Promise<LoginResponse> {
  const response = await api.post<LoginResponse>('/auth/register', data, { skipAuth: true });
  setTokens(response.access_token, response.refresh_token);
  return response;
}

export async function logout(): Promise<void> {
  try {
    await api.post('/auth/logout');
  } finally {
    clearTokens();
  }
}

export async function getMe(): Promise<MeResponse> {
  return api.get<MeResponse>('/auth/me');
}
