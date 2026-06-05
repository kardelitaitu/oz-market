if (import.meta.env.VITE_BACKEND_URL) {
  window.__BACKEND_URL = import.meta.env.VITE_BACKEND_URL;
}

import { mount } from 'svelte'
import './global.css'
import App from './App.svelte'

const app = mount(App, {
  target: document.getElementById('app'),
})

export default app
