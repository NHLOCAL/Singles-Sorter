import {describe,it,expect} from 'vitest';
import {HelpState} from '../src/help-state';
describe('input-specific explanation lifetime',()=>{
 it('hides mouse hover synchronously when the badge is left',()=>{const s=new HelpState<string>();s.enter('source','mouse');expect(s.active).toBe('source');s.leave('source','mouse');expect(s.active).toBeNull();});
 it('mouse clicks and repeated clicks cannot pin hover help',()=>{const s=new HelpState<string>();s.enter('source','mouse');s.focus('source',true);s.activate('source','hover');s.activate('source','hover');s.leave('source','mouse');expect(s.active).toBeNull();});
 it('pen hover uses the same immediate leave behavior',()=>{const s=new HelpState<string>();s.enter('source','pen');s.leave('source','pen');expect(s.active).toBeNull();});
 it('keyboard focus remains until blur or explicit dismissal',()=>{const s=new HelpState<string>();s.focus('source',false);s.enter('source','mouse');s.leave('source','mouse');expect(s.origin).toBe('keyboard');s.blur('source');expect(s.active).toBeNull();});
 it('keyboard activation reopens after Escape without moving focus',()=>{const s=new HelpState<string>();s.focus('source',false);s.close();s.activate('source','keyboard');expect(s.active).toBe('source');s.close();expect(s.active).toBeNull();});
 it('touch ignores synthetic enter/leave and toggles on each tap',()=>{const s=new HelpState<string>();s.enter('source','touch');s.focus('source',true);expect(s.active).toBeNull();s.activate('source','touch');s.leave('source','touch');expect(s.active).toBe('source');s.activate('source','touch');expect(s.active).toBeNull();});
 it('outside interaction, focus loss and dialog closure can always dismiss touch help',()=>{const s=new HelpState<string>();s.activate('source','touch');s.close();expect(s.active).toBeNull();s.activate('source','touch');s.blur('source');expect(s.active).toBeNull();});
 it('a previous badge leaving cannot hide the next badge',()=>{const s=new HelpState<string>();s.enter('source','mouse');s.enter('target','mouse');s.leave('source','mouse');expect(s.active).toBe('target');s.leave('target','mouse');expect(s.active).toBeNull();});
});
