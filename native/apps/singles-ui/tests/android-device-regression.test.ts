import {describe,it,expect,vi} from 'vitest';
import {executionRefusalSummary,executionRefusalDetails} from '../src/result-presentation';
import {PreviewController} from '../src/state';
import {defaults} from '../src/contracts';
import type {Host,ApplyReadiness} from '../src/contracts';

describe('physical Android regressions',()=>{
 it('does not report an empty original plan as a permission/identity failure',()=>{
  const readiness:ApplyReadiness={no_operations:true,supported:false,operation_count:0,copy_mode:true,source:'songs',target:'empty',reason:'לא זוהו אמנים למיון'};
  const blockers=['MISSING_IDENTITY','PHYSICAL_APPLY_UNAVAILABLE','UNKNOWN_CAPABILITY'];
  expect(executionRefusalSummary(readiness,blockers)).toBe(readiness.reason);
  expect(executionRefusalDetails(readiness,blockers)).toBe('');
  expect(executionRefusalSummary({...readiness,no_operations:false},blockers)).toContain('זהות');
 });
 it('restores the native AI preference and saves explicit changes before another job',async()=>{
  let finish!:(value:'Disabled'|'Configured')=>void;
  const save=vi.fn(()=>new Promise<'Disabled'|'Configured'>(resolve=>{finish=resolve}));
  const host={getSettings:async()=>({...defaults}),getModelStatus:async()=>({available:true,platform:'android',message:'verified',preferred_ai:'Configured'}),saveAiProfile:save} as unknown as Host;
  const controller=new PreviewController(host);await controller.initialize();
  expect(controller.state.ai).toBe('Configured');
  const pending=controller.changeAi('Disabled');expect(controller.state.busy).toBe(true);
  finish('Disabled');await pending;expect(save).toHaveBeenCalledWith('Disabled');
  expect(controller.state.busy).toBe(false);expect(controller.state.ai).toBe('Disabled');
 });
});
