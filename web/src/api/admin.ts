import { api, buildQueryString, type PaginationParams } from './client';
import type { AdminUser, AdminBoard, AdminFolder, PaginatedResponse } from '../types';

// User management
export async function getUsers(
  params?: PaginationParams
): Promise<PaginatedResponse<AdminUser>> {
  const query = buildQueryString(params || {});
  return api.get<PaginatedResponse<AdminUser>>(`/admin/users${query}`);
}

export async function getUser(id: number): Promise<AdminUser> {
  return api.get<AdminUser>(`/admin/users/${id}`);
}

export interface UpdateUserRequest {
  nickname?: string;
  email?: string;
  role?: string;
  is_active?: boolean;
}

export async function updateUser(id: number, data: UpdateUserRequest): Promise<AdminUser> {
  return api.put<AdminUser>(`/admin/users/${id}`, data);
}

export async function deleteUser(id: number): Promise<void> {
  await api.delete(`/admin/users/${id}`);
}

// Board management
export async function getAdminBoards(): Promise<AdminBoard[]> {
  return api.get<AdminBoard[]>('/admin/boards');
}

export interface CreateBoardRequest {
  name: string;
  description?: string;
  board_type: string;
  permission: string;
  post_permission: string;
}

export async function createBoard(data: CreateBoardRequest): Promise<AdminBoard> {
  return api.post<AdminBoard>('/admin/boards', data);
}

export interface UpdateBoardRequest {
  name?: string;
  description?: string;
  permission?: string;
  post_permission?: string;
  order_num?: number;
  is_visible?: boolean;
}

export async function updateBoard(id: number, data: UpdateBoardRequest): Promise<AdminBoard> {
  return api.put<AdminBoard>(`/admin/boards/${id}`, data);
}

export async function deleteBoard(id: number): Promise<void> {
  await api.delete(`/admin/boards/${id}`);
}

// Folder management
export async function getAdminFolders(): Promise<AdminFolder[]> {
  return api.get<AdminFolder[]>('/admin/folders');
}

export interface CreateFolderRequest {
  name: string;
  description?: string;
  parent_id?: number;
  permission: string;
  upload_perm: string;
}

export async function createFolder(data: CreateFolderRequest): Promise<AdminFolder> {
  return api.post<AdminFolder>('/admin/folders', data);
}

export interface UpdateFolderRequest {
  name?: string;
  description?: string;
  permission?: string;
  upload_perm?: string;
  order_num?: number;
}

export async function updateFolder(id: number, data: UpdateFolderRequest): Promise<AdminFolder> {
  return api.put<AdminFolder>(`/admin/folders/${id}`, data);
}

export async function deleteFolder(id: number): Promise<void> {
  await api.delete(`/admin/folders/${id}`);
}
