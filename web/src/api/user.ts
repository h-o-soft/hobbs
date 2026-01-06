import { api, buildQueryString } from './client';
import type { PaginatedResponse } from '../types';

export interface UserListResponse {
  id: number;
  username: string;
  nickname: string;
  role: string;
  last_login_at?: string;
}

export interface UserDetailResponse {
  id: number;
  username: string;
  nickname: string;
  role: string;
  profile?: string;
  created_at: string;
  last_login_at?: string;
}

export interface UpdateProfileRequest {
  nickname?: string;
  email?: string;
  profile?: string;
}

export interface ChangePasswordRequest {
  current_password: string;
  new_password: string;
}

export interface PaginationParams {
  page?: number;
  per_page?: number;
}

/**
 * Get user list (paginated).
 */
export async function listUsers(params: PaginationParams = {}): Promise<PaginatedResponse<UserListResponse>> {
  const query = buildQueryString(params as Record<string, number | undefined>);
  return api.get<PaginatedResponse<UserListResponse>>(`/users${query}`);
}

/**
 * Get current user's profile.
 */
export async function getMyProfile(): Promise<UserDetailResponse> {
  return api.get<UserDetailResponse>('/users/me');
}

/**
 * Update current user's profile.
 */
export async function updateMyProfile(data: UpdateProfileRequest): Promise<UserDetailResponse> {
  return api.put<UserDetailResponse>('/users/me', data);
}

/**
 * Change current user's password.
 */
export async function changePassword(data: ChangePasswordRequest): Promise<void> {
  await api.post<void>('/users/me/password', data);
}

/**
 * Get user profile by ID.
 */
export async function getUserById(id: number): Promise<UserDetailResponse> {
  return api.get<UserDetailResponse>(`/users/${id}`);
}

/**
 * Get user profile by username.
 */
export async function getUserByUsername(username: string): Promise<UserDetailResponse> {
  return api.get<UserDetailResponse>(`/users/by-username/${encodeURIComponent(username)}`);
}
