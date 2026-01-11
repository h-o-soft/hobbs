import { api } from './client';

export interface SiteConfig {
  name: string;
  description: string;
  sysop_name: string;
  telnet_enabled: boolean;
}

/**
 * Get public site configuration.
 * This endpoint does not require authentication.
 */
export async function getPublicConfig(): Promise<SiteConfig> {
  return api.get<SiteConfig>('/config/public', { skipAuth: true });
}
