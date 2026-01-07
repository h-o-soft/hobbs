import { createSignal, createContext, useContext, createEffect, type ParentComponent } from 'solid-js';
import type { SiteConfig } from '../api/config';
import * as configApi from '../api/config';

interface SiteConfigState {
  config: SiteConfig;
  isLoading: boolean;
}

interface SiteConfigActions {
  loadConfig: () => Promise<void>;
}

type SiteConfigContextValue = [SiteConfigState, SiteConfigActions];

const SiteConfigContext = createContext<SiteConfigContextValue>();

const DEFAULT_CONFIG: SiteConfig = {
  name: 'HOBBS',
  description: 'A retro BBS system',
  sysop_name: 'SysOp',
};

export const SiteConfigProvider: ParentComponent = (props) => {
  const [config, setConfig] = createSignal<SiteConfig | null>(null);
  const [isLoading, setIsLoading] = createSignal(true);

  const state = {
    get config() { return config() ?? DEFAULT_CONFIG; },
    get isLoading() { return isLoading(); },
  };

  const actions: SiteConfigActions = {
    async loadConfig() {
      try {
        const siteConfig = await configApi.getPublicConfig();
        setConfig(siteConfig);
        // Update document title
        document.title = siteConfig.name;
      } catch (e) {
        console.error('Failed to load site config:', e);
        // Use default config on error
        setConfig(DEFAULT_CONFIG);
      } finally {
        setIsLoading(false);
      }
    },
  };

  // Load config on mount
  createEffect(() => {
    actions.loadConfig();
  });

  return (
    <SiteConfigContext.Provider value={[state, actions]}>
      {props.children}
    </SiteConfigContext.Provider>
  );
};

export function useSiteConfig(): SiteConfigContextValue {
  const context = useContext(SiteConfigContext);
  if (!context) {
    throw new Error('useSiteConfig must be used within a SiteConfigProvider');
  }
  return context;
}
