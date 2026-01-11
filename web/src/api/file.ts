import { api, buildQueryString, type PaginationParams } from './client';
import { getOneTimeToken } from './auth';
import type { Folder, FileInfo, PaginatedResponse } from '../types';

export async function getFolders(): Promise<Folder[]> {
  return api.get<Folder[]>('/folders');
}

export async function getFolder(id: number): Promise<Folder> {
  return api.get<Folder>(`/folders/${id}`);
}

export async function getFiles(
  folderId: number,
  params?: PaginationParams
): Promise<PaginatedResponse<FileInfo>> {
  const query = buildQueryString(params || {});
  return api.get<PaginatedResponse<FileInfo>>(`/folders/${folderId}/files${query}`);
}

export async function getFileInfo(id: number): Promise<FileInfo> {
  return api.get<FileInfo>(`/files/${id}`);
}

export async function uploadFile(
  folderId: number,
  file: File,
  description?: string
): Promise<FileInfo> {
  const formData = new FormData();
  formData.append('file', file);
  if (description) {
    formData.append('description', description);
  }
  return api.post<FileInfo>(`/folders/${folderId}/files`, formData);
}

export async function deleteFile(id: number): Promise<void> {
  await api.delete(`/files/${id}`);
}

export async function downloadFile(id: number, filename: string): Promise<void> {
  // Get one-time token for download
  const tokenResponse = await getOneTimeToken('download', id);
  const url = `/api/files/${id}/download-with-token?token=${encodeURIComponent(tokenResponse.token)}`;

  // Create a temporary link and trigger download
  const link = document.createElement('a');
  link.href = url;
  link.download = filename;
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
}
