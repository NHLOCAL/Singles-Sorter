import {describe,it,expect} from 'vitest';
import {kindLabel,reasonLabel,blockerLabel} from '../src/presentation';
import actualHostPage from '../src-tauri/tests/fixtures/real-preview-page.json';
import type {PlanRow} from '../src/contracts';
const row:PlanRow={id:'1',source:'שיר.mp3',destination:null,kind:'FixNames',reason:'rename_intended=false; save_intended=true; repairs=0; failure=None',blockers:[]};
describe('display language does not change engine codes',()=>{
 it('presents actual serialized host rows without changing paths or chosen singers',()=>{expect(actualHostPage.total).toBe(2);for(const row of actualHostPage.rows){expect(kindLabel(row.kind)).toMatch(/[א-ת]/);expect(reasonLabel(row)).toMatch(/[א-ת]/);}expect(actualHostPage.rows[1].destination).toBe('אברהם פריד / סינגלים / אברהם פריד - בדיקה.mp3');});
 it('translates all known kinds',()=>{for(const kind of ['Album','Single','FixNames','ItemError','Vanished'])expect(kindLabel(kind)).toMatch(/[א-ת]/);});
 it('does not expose debug formatting as the main explanation',()=>{expect(reasonLabel(row)).not.toContain('rename_intended');expect(row.reason).toContain('rename_intended');});
 it('keeps chosen singer strings untouched',()=>{expect(reasonLabel({...row,kind:'Single',destination:'זמר/שיר.mp3',reason:'זמר א; זמר ב'})).toContain('זמר א; זמר ב');});
 it('does not invent an unknown blocker cause',()=>{expect(blockerLabel('NEW_CODE')).toContain('הסיבה הטכנית');});
});
