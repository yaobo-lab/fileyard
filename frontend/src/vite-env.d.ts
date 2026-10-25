/// <reference types="vite/client" />

interface AppRuntimeConfig {
  APP_NAME?: string;
  API_URL?: string;
  DEBUG?: boolean;
  [key: string]: unknown;
}

interface Window {
  __APP_CONFIG__?: AppRuntimeConfig;
}
