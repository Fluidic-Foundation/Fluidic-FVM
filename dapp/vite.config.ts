import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  base: process.env.DAPP_BASE || '/dapp/',
  plugins: [react()],
})
