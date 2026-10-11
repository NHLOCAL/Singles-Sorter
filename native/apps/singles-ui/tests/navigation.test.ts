import {describe,it,expect} from 'vitest';
import {WorkspaceNavigation} from '../src/navigation';
describe('task-focused presentation navigation',()=>{
 it('goes setup → processing → results without changing mode on duplicate start',()=>{const n=new WorkspaceNavigation();expect(n.start('FixNames')).toBe(true);expect(n.start('Sort')).toBe(false);expect(n.mode).toBe('FixNames');n.finish(true);expect(n.screen).toBe('results');});
 it('returns to setup and reopens existing results without starting work',()=>{const n=new WorkspaceNavigation();n.start('Sort');n.finish(true);n.setup();expect(n.screen).toBe('setup');n.results(true);expect(n.screen).toBe('results');});
 it('does not show stale/missing results, and failed processing returns to setup',()=>{const n=new WorkspaceNavigation();n.results(false);expect(n.screen).toBe('setup');n.start('Sort');n.setup();expect(n.screen).toBe('processing');n.finish(false);expect(n.screen).toBe('setup');});
});
