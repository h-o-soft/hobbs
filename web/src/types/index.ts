// API Response types

export interface ApiResponse<T> {
  success: boolean;
  data: T;
  error?: string;
}

export interface PaginatedResponse<T> {
  data: T[];
  page: number;
  per_page: number;
  total: number;
  total_pages: number;
}

// User types
export interface UserInfo {
  id: number;
  username: string;
  nickname: string;
  role: string;
}

export interface MeResponse extends UserInfo {
  email?: string;
  unread_mail_count: number;
  created_at: string;
  last_login_at?: string;
}

// Auth types
export interface LoginRequest {
  username: string;
  password: string;
}

export interface LoginResponse {
  access_token: string;
  refresh_token: string;
  expires_in: number;
  user: UserInfo;
}

export interface RegisterRequest {
  username: string;
  password: string;
  nickname: string;
  email?: string;
}

export interface RefreshRequest {
  refresh_token: string;
}

export interface RefreshResponse {
  access_token: string;
  refresh_token: string;
  expires_in: number;
}

// Board types
export interface Board {
  id: number;
  name: string;
  description?: string;
  board_type: 'thread' | 'flat';
  thread_count: number;
  post_count: number;
  can_post: boolean;
  created_at: string;
}

export interface Thread {
  id: number;
  board_id: number;
  title: string;
  author: AuthorInfo;
  post_count: number;
  created_at: string;
  updated_at: string;
}

export interface Post {
  id: number;
  thread_id?: number;
  board_id?: number;
  title?: string;
  content: string;
  author: AuthorInfo;
  created_at: string;
}

export interface AuthorInfo {
  id: number;
  username: string;
  nickname: string;
}

export interface CreateThreadRequest {
  title: string;
  content: string;
}

export interface CreatePostRequest {
  content: string;
}

// Mail types
export interface Mail {
  id: number;
  from_user: AuthorInfo;
  to_user: AuthorInfo;
  subject: string;
  content: string;
  is_read: boolean;
  created_at: string;
}

export interface MailListItem {
  id: number;
  from_user?: AuthorInfo;
  to_user?: AuthorInfo;
  subject: string;
  is_read: boolean;
  created_at: string;
}

export interface SendMailRequest {
  to_username: string;
  subject: string;
  content: string;
}

// Chat types
export interface ChatRoom {
  id: string;
  name: string;
  participant_count: number;
}

export interface ChatParticipant {
  user_id?: number;
  username: string;
}

// WebSocket message types
export type ClientMessage =
  | { type: 'join'; room_id: string }
  | { type: 'leave' }
  | { type: 'message'; content: string }
  | { type: 'action'; content: string }
  | { type: 'ping' };

export type ServerMessage =
  | { type: 'chat'; user_id?: number; username: string; content: string; timestamp: string }
  | { type: 'action'; user_id: number; username: string; content: string; timestamp: string }
  | { type: 'user_joined'; user_id: number; username: string; timestamp: string }
  | { type: 'user_left'; user_id: number; username: string; timestamp: string }
  | { type: 'system'; content: string; timestamp: string }
  | { type: 'error'; code: string; message: string }
  | { type: 'pong' }
  | { type: 'joined'; room_id: string; room_name: string; participants: ChatParticipant[] }
  | { type: 'left'; room_id: string }
  | { type: 'room_list'; rooms: ChatRoom[] };

// File types
export interface Folder {
  id: number;
  name: string;
  description?: string;
  parent_id?: number;
  can_read: boolean;
  can_upload: boolean;
  file_count: number;
  created_at: string;
}

export interface FileInfo {
  id: number;
  folder_id: number;
  filename: string;
  size: number;
  description?: string;
  uploader: AuthorInfo;
  downloads: number;
  created_at: string;
}

// RSS types
export interface RssFeed {
  id: number;
  url: string;
  title: string;
  description?: string;
  unread_count: number;
  last_updated?: string;
  error_count: number;
  last_error?: string;
}

export interface RssItem {
  id: number;
  feed_id: number;
  title: string;
  link?: string;
  description?: string;
  pub_date?: string;
  is_read: boolean;
}

// Admin types
export interface AdminUser {
  id: number;
  username: string;
  nickname: string;
  email?: string;
  role: string;
  is_active: boolean;
  created_at: string;
  last_login?: string;
}

export interface AdminBoard {
  id: number;
  name: string;
  description?: string;
  board_type: string;
  permission: string;
  post_permission: string;
  order_num: number;
  is_visible: boolean;
}

export interface AdminFolder {
  id: number;
  name: string;
  description?: string;
  parent_id?: number;
  permission: string;
  upload_perm: string;
  order_num: number;
}
