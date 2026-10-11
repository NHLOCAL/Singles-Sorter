import {describe,it,expect} from 'vitest';
import config from '../src-tauri/tauri.conf.json';
import cargo from '../src-tauri/Cargo.toml?raw';
import host from '../src-tauri/src/job_owner.rs?raw';
import {APP_VERSION,PROJECT_LINKS} from '../src/about';
describe('verified About identity',()=>{
 it('uses the packaged version and matches both native version declarations',()=>{
  expect(APP_VERSION).toBe(config.version);
  expect(cargo).toContain(`version = "${APP_VERSION}"`);
 });
 it('offers only verified project destinations, also pinned in native code',()=>{
  expect(PROJECT_LINKS).toEqual({Source:'https://github.com/NHLOCAL/Singles-Sorter',License:'https://github.com/NHLOCAL/Singles-Sorter/blob/main/LICENSE'});
  for(const url of Object.values(PROJECT_LINKS))expect(host).toContain(`"${url}"`);
 });
});
