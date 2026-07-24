import type { Config } from 'tailwindcss'
export default {
  content: ['./index.html', './src/**/*.{vue,ts,tsx}'],
  darkMode: 'class',
  theme: { extend: {
    colors: { reader: { paper: '#f8f5e9', sepia: '#f4ecd8', dark: '#1a1a1a' } },
  }},
  plugins: [],
} satisfies Config
