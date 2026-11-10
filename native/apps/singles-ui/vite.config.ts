import { defineConfig } from 'vite';
export default defineConfig(({mode})=>({clearScreen:false,server:{port:1420,strictPort:true,host:'127.0.0.1'},build:{target:mode==='android'?'chrome83':'es2022'},envPrefix:['VITE_','TAURI_ENV_']}));
