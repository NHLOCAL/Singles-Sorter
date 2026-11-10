import {describe,it,expect,vi} from 'vitest';
import {FirstRunGate} from '../src/first-run';
describe('first-run explicit acknowledgement',()=>{
 it('loading/showing does not mark seen; app exit has no acknowledgement call',async()=>{const host={getFirstRunStatus:async()=>({version:'v1',seen:false}),markFirstRunSeen:vi.fn(async()=>({version:'v1',seen:true}))};const gate=new FirstRunGate(host);expect(await gate.load()).toBe(true);expect(host.markFirstRunSeen).not.toHaveBeenCalled();});
 it('explicit acknowledgement writes once',async()=>{const host={getFirstRunStatus:async()=>({version:'v1',seen:false}),markFirstRunSeen:vi.fn(async()=>({version:'v1',seen:true}))};const gate=new FirstRunGate(host);await gate.load();await Promise.all([gate.acknowledge(),gate.acknowledge()]);await gate.acknowledge();expect(host.markFirstRunSeen).toHaveBeenCalledTimes(1);expect(gate.pending).toBe(false);});
 it('known version does not ask for acknowledgement',async()=>{const host={getFirstRunStatus:async()=>({version:'v1',seen:true}),markFirstRunSeen:vi.fn(async()=>({version:'v1',seen:true}))};const gate=new FirstRunGate(host);expect(await gate.load()).toBe(false);await gate.acknowledge();expect(host.markFirstRunSeen).not.toHaveBeenCalled();});
 it('failed write retains unseen state',async()=>{const gate=new FirstRunGate({getFirstRunStatus:async()=>({version:'v1',seen:false}),markFirstRunSeen:async()=>{throw new Error('disk');}});await gate.load();await expect(gate.acknowledge()).rejects.toThrow('disk');expect(gate.pending).toBe(true);});
});
