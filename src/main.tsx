import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './index.css'
import { App } from './App.tsx'

const wdioReady = (import.meta as ImportMeta & { env?: { VITE_E2E?: string } }).env?.VITE_E2E === '1'
  ? import('@wdio/tauri-plugin')
  : Promise.resolve()

// Apply the persisted theme before React mounts to avoid a light/dark flash.
try {
  const saved = JSON.parse(localStorage.getItem('searchis_config_v1') ?? '{}') as { theme?: string }
  const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches
  const isDark = saved.theme === 'dark' || ((saved.theme === 'system' || !saved.theme) && prefersDark)
  document.documentElement.classList.toggle('dark', isDark)
  document.documentElement.setAttribute('data-theme', isDark ? 'dark' : 'light')
} catch {
  const isDark = window.matchMedia('(prefers-color-scheme: dark)').matches
  document.documentElement.classList.toggle('dark', isDark)
  document.documentElement.setAttribute('data-theme', isDark ? 'dark' : 'light')
}

void wdioReady.finally(() => {
  createRoot(document.getElementById('root')!).render(
    <StrictMode>
      <App />
    </StrictMode>,
  )
})
