import { api, buildQueryString, type PaginationParams } from './client';
import type {
  Board,
  Thread,
  Post,
  PaginatedResponse,
  CreateThreadRequest,
  CreatePostRequest,
  CreateFlatPostRequest,
} from '../types';

export async function getBoards(): Promise<Board[]> {
  return api.get<Board[]>('/boards');
}

export async function getBoard(id: number): Promise<Board> {
  return api.get<Board>(`/boards/${id}`);
}

export async function getThreads(
  boardId: number,
  params?: PaginationParams
): Promise<PaginatedResponse<Thread>> {
  const query = buildQueryString(params || {});
  return api.get<PaginatedResponse<Thread>>(`/boards/${boardId}/threads${query}`);
}

export async function createThread(
  boardId: number,
  data: CreateThreadRequest
): Promise<Thread> {
  return api.post<Thread>(`/boards/${boardId}/threads`, data);
}

export async function getThread(threadId: number): Promise<Thread> {
  return api.get<Thread>(`/threads/${threadId}`);
}

export async function getPosts(
  threadId: number,
  params?: PaginationParams
): Promise<PaginatedResponse<Post>> {
  const query = buildQueryString(params || {});
  return api.get<PaginatedResponse<Post>>(`/threads/${threadId}/posts${query}`);
}

export async function createPost(
  threadId: number,
  data: CreatePostRequest
): Promise<Post> {
  return api.post<Post>(`/threads/${threadId}/posts`, data);
}

// Flat board posts (for flat-type boards)
export async function getFlatPosts(
  boardId: number,
  params?: PaginationParams
): Promise<PaginatedResponse<Post>> {
  const query = buildQueryString(params || {});
  return api.get<PaginatedResponse<Post>>(`/boards/${boardId}/posts${query}`);
}

export async function createFlatPost(
  boardId: number,
  data: CreateFlatPostRequest
): Promise<Post> {
  return api.post<Post>(`/boards/${boardId}/posts`, data);
}

export async function deletePost(postId: number): Promise<void> {
  return api.delete(`/posts/${postId}`);
}

export interface UpdatePostRequest {
  title?: string;
  body: string;
}

export async function updatePost(
  postId: number,
  data: UpdatePostRequest
): Promise<Post> {
  return api.patch<Post>(`/posts/${postId}`, data);
}
