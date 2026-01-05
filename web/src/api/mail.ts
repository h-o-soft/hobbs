import { api, buildQueryString, type PaginationParams } from './client';
import type {
  Mail,
  MailListItem,
  SendMailRequest,
  PaginatedResponse,
} from '../types';

export async function getInbox(
  params?: PaginationParams
): Promise<PaginatedResponse<MailListItem>> {
  const query = buildQueryString(params || {});
  return api.get<PaginatedResponse<MailListItem>>(`/mail/inbox${query}`);
}

export async function getSent(
  params?: PaginationParams
): Promise<PaginatedResponse<MailListItem>> {
  const query = buildQueryString(params || {});
  return api.get<PaginatedResponse<MailListItem>>(`/mail/sent${query}`);
}

export async function getMail(id: number): Promise<Mail> {
  return api.get<Mail>(`/mail/${id}`);
}

export async function sendMail(data: SendMailRequest): Promise<Mail> {
  return api.post<Mail>('/mail', data);
}

export async function deleteMail(id: number): Promise<void> {
  await api.delete(`/mail/${id}`);
}

export async function markAsRead(id: number): Promise<void> {
  await api.put(`/mail/${id}/read`);
}
