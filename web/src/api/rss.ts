import { api, buildQueryString, type PaginationParams } from './client';
import type { RssFeed, RssItem, PaginatedResponse } from '../types';

export async function getFeeds(): Promise<RssFeed[]> {
  return api.get<RssFeed[]>('/rss/feeds');
}

export async function getFeed(id: number): Promise<RssFeed> {
  return api.get<RssFeed>(`/rss/feeds/${id}`);
}

export interface AddFeedRequest {
  url: string;
  title?: string;
}

export async function addFeed(data: AddFeedRequest): Promise<RssFeed> {
  return api.post<RssFeed>('/rss/feeds', data);
}

export async function deleteFeed(id: number): Promise<void> {
  await api.delete(`/rss/feeds/${id}`);
}

export async function refreshFeed(id: number): Promise<RssFeed> {
  return api.post<RssFeed>(`/rss/feeds/${id}/refresh`);
}

export async function refreshAllFeeds(): Promise<void> {
  await api.post('/rss/feeds/refresh');
}

export async function getItems(
  feedId: number,
  params?: PaginationParams & { unread_only?: boolean }
): Promise<PaginatedResponse<RssItem>> {
  const query = buildQueryString(params || {});
  return api.get<PaginatedResponse<RssItem>>(`/rss/feeds/${feedId}/items${query}`);
}

export async function getItem(id: number): Promise<RssItem> {
  return api.get<RssItem>(`/rss/items/${id}`);
}

export async function markItemAsRead(id: number): Promise<void> {
  await api.put(`/rss/items/${id}/read`);
}

export async function markAllAsRead(feedId: number): Promise<void> {
  await api.put(`/rss/feeds/${feedId}/read-all`);
}
