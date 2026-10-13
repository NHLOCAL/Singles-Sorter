"""Mode-dependent controls remain real rendered inputs, never unhidden by tests."""
from pathlib import Path
import json
import subprocess
import unittest

from desktop_modes import select_sort_setup, SORT_CONTROLS_READY, SORT_TAB_READY


class ModeSetupTests(unittest.TestCase):
    def test_repeat_fix_and_following_sort_use_real_tab_before_editing_target(self):
        for initial in ['fix', 'sort']:
            state = {'mode':initial, 'setup':True, 'busy':False}
            events = []
            def script(source):
                events.append('read')
                if source == SORT_CONTROLS_READY:
                    return state['setup'] and not state['busy'] and state['mode']=='sort'
                return state['setup'] and not state['busy']
            def click(selector):
                self.assertEqual(selector, '#tab-mode-sort')
                events.append('real-sort-tab'); state['mode']='sort'
            def wait(predicate):
                self.assertTrue(predicate())
            select_sort_setup(script, click, wait)
            self.assertEqual(events, ['read','real-sort-tab','read'])
            self.assertEqual(state['mode'],'sort')

    def test_busy_or_non_setup_view_refuses_before_mode_click(self):
        for state in ['busy', 'results', 'open modal']:
            clicks = []
            def wait(predicate):
                if not predicate(): raise TimeoutError(state)
            with self.assertRaises(TimeoutError):
                select_sort_setup(lambda _:False, clicks.append, wait)
            self.assertEqual(clicks, [])

    def test_unsettled_or_hidden_controls_do_not_complete_setup(self):
        clicks = []
        def script(source):
            return source != SORT_CONTROLS_READY
        def wait(predicate):
            if not predicate(): raise TimeoutError('target still hidden')
        with self.assertRaisesRegex(TimeoutError, 'hidden'):
            select_sort_setup(script, clicks.append, wait)
        self.assertEqual(clicks, ['#tab-mode-sort'])
        for expected in ["dataset.workMode==='sort'", 'element.disabled',
                         "'target-picker'", "'copy_mode'", "'exist_only'",
                         "getAttribute('aria-pressed')==='true'", "querySelector('dialog[open]')",
                         "style.visibility!=='visible'", 'Number(style.opacity)===0']:
            self.assertIn(expected, SORT_CONTROLS_READY)
        self.assertIn("!document.querySelector('dialog[open]')", SORT_TAB_READY)
        self.assertNotIn('.hidden=', SORT_CONTROLS_READY)
        self.assertNotIn('dispatchEvent', SORT_CONTROLS_READY)

    def test_real_readiness_predicate_refuses_hidden_disabled_or_modal_controls(self):
        # Execute only the read-only predicate against authored DOM-shaped data.
        # Actual Windows rendering remains the runtime acceptance gate.
        program = r"""
const predicate=new Function(JSON.parse(process.argv[1]));
function trial(change){
 const nodes={};
 for(const id of ['source','source-picker','target','target-picker','copy_mode','main_folder_only','singles_folder','abc_sort','exist_only','sort','tab-mode-sort']){
  nodes[id]={disabled:false,parentElement:null,style:{visibility:'visible',opacity:'1'},getClientRects(){return this.hidden?[]:[{}]},getAttribute(){return 'true'}};
 }
 nodes.app={dataset:{view:'setup',workMode:'sort'}};
 let modal=false;
 global.document={querySelector(s){return s==='dialog[open]'?(modal?{}:null):nodes[s.slice(1)]},getElementById(id){return nodes[id]}};
 global.getComputedStyle=n=>n.style;
 change(nodes,()=>modal=true);
 return predicate();
}
console.log(JSON.stringify([
 trial(()=>{}),trial(n=>n.target.hidden=true),trial(n=>n['target-picker'].disabled=true),
 trial(n=>n.copy_mode.style.visibility='hidden'),trial(n=>n.exist_only.style.opacity='0'),
 trial(n=>n.abc_sort.parentElement={parentElement:null,style:{visibility:'collapse',opacity:'1'}}),
 trial(n=>n['tab-mode-sort'].disabled=true),trial(n=>n.app.dataset.workMode='fix'),
 trial((n,open)=>open())
]));
"""
        completed = subprocess.run(['node', '-e', program, json.dumps(SORT_CONTROLS_READY)],
                                   check=True, capture_output=True, text=True)
        self.assertEqual(json.loads(completed.stdout), [True] + [False] * 8)

    def test_initial_default_and_picker_settings_recovery_boundaries(self):
        root = Path(__file__).parent
        main = (root / 'desktop.py').read_text(encoding='utf-8')
        self.assertIn("assert script(\"return document.querySelector('#app').dataset.workMode==='sort'\")", main)
        pickers = main.split("fixture_guard.verify('before-native-picker')", 1)[1].split("checks.append('Native folder dialogs", 1)[0]
        self.assertLess(pickers.index('select_sort_setup('), pickers.index("pick_owned_path('#source-picker'"))
        self.assertLess(pickers.index('select_sort_setup('), pickers.index("pick_owned_path('#target-picker'"))
        # Main settings have their own guard; dialogs retain their own controls.
        settings = (root / 'desktop_settings.py').read_text(encoding='utf-8').split('def set_main_settings(',1)[1].split('def editor_rows(',1)[0]
        self.assertLess(settings.index('select_sort_setup('), settings.index('for key in MAIN_SETTINGS:'))
        for name in ['desktop_recovery.py','desktop_album_recovery.py','desktop_whole_album_recovery.py']:
            recovery = (root / name).read_text(encoding='utf-8')
            for selector in ['#target', '#copy_mode', '#singles_folder', '#abc_sort', '#exist_only']:
                self.assertNotIn("fill('"+selector+"'", recovery)
                self.assertNotIn("click('"+selector+"'", recovery)

    def test_every_target_edit_is_preceded_by_mode_setup_including_all_formats(self):
        expected={'desktop.py','desktop_ai.py','desktop_apply.py','desktop_albums.py',
                  'desktop_copied_move.py','desktop_cross_albums.py','desktop_stability.py',
                  'desktop_repair.py','desktop_flac.py','desktop_ogg.py','desktop_mp4.py',
                  'desktop_id3_container.py','desktop_wavpack.py','desktop_metadata_readiness.py'}
        found=set()
        for path in Path(__file__).parent.glob('desktop*.py'):
            lines=path.read_text(encoding='utf-8').splitlines()
            for index,line in enumerate(lines):
                if "fill('#target'" not in line:
                    continue
                found.add(path.name)
                self.assertTrue(any('select_sort_setup(' in previous for previous in lines[max(0,index-2):index]),
                                f'{path.name}:{index+1} edits a potentially hidden target')
        self.assertEqual(found, expected)
        for name in ['desktop_repair.py','desktop_flac.py','desktop_ogg.py','desktop_mp4.py',
                     'desktop_id3_container.py','desktop_wavpack.py']:
            source=(Path(__file__).parent/name).read_text(encoding='utf-8')
            setup=source.split('select_sort_setup(script, click, wait)',1)[1]
            self.assertIn("fill('#target', '')",setup)
            self.assertLess(setup.index("fill('#target', '')"),setup.index("click('#tab-mode-fix')"))
            self.assertLess(setup.index("click('#tab-mode-fix')"),setup.index("click('#fix')"))


if __name__ == '__main__':
    unittest.main()
