import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './index.css'
import { App } from './App.tsx'

// Apply the persisted theme before React mounts to avoid a light/dark flash.
try {
  const saved = JSON.parse(localStorage.getItem('searchis_config_v1') ?? '{}') as { theme?: string }
  const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches
  document.documentElement.classList.toggle(
    'dark',
    saved.theme === 'dark' || ((saved.theme === 'system' || !saved.theme) && prefersDark),
  )
} catch {
  document.documentElement.classList.toggle('dark', window.matchMedia('(prefers-color-scheme: dark)').matches)
}

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
)
