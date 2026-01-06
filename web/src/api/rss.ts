import { api, buildQueryString, type PaginationParams } from './client';
import type { RssFeed, RssItem, PaginatedResponse } from '../types';

// Personal RSS Reader APIs
// Each user manages their own feed subscriptions

export async function getFeeds(): Promise<RssFeed[]> {
  return api.get<RssFeed[]>('/rss');
}

export async function getFeed(id: number): Promise<RssFeed> {
  return api.get<RssFeed>(`/rss/${id}`);
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

export async function getItems(
  feedId: number,
  params?: PaginationParams & { unread_only?: boolean }
): Promise<PaginatedResponse<RssItem>> {
  const query = buildQueryString(params || {});
  return api.get<PaginatedResponse<RssItem>>(`/rss/${feedId}/items${query}`);
}

export async function getItem(feedId: number, itemId: number): Promise<RssItem> {
  return api.get<RssItem>(`/rss/${feedId}/items/${itemId}`);
}

export async function markAllAsRead(feedId: number): Promise<void> {
  await api.post(`/rss/${feedId}/mark-read`);
}
