import {describe,it,expect} from 'vitest';
import {viewportHeight} from '../src/viewport';
describe('bounded viewport height',()=>{
 it('uses the visible viewport when an on-screen keyboard reduces it',()=>{expect(viewportHeight(800,{height:420,scale:1})).toBe(420);});
 it('does not reflow underneath pinch magnification',()=>{expect(viewportHeight(800,{height:400,scale:2})).toBe(800);});
 it('uses the normal viewport without the newer API or valid measurements',()=>{expect(viewportHeight(650,null)).toBe(650);expect(viewportHeight(650,{height:NaN,scale:1})).toBe(650);expect(viewportHeight(650,{height:0,scale:1})).toBe(650);});
});
