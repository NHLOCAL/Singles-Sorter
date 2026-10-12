"""Actual native-WebView UI checks; never invokes a real external browser."""
import json
from desktop_layout import check_phone_layout
ELEMENT = 'element-6066-11e4-a52e-4f735466cecf'

def exercise_information(command, script, click, wait, capture, root, output):
    def hover(selector):
        element = command('POST', '/element', {'using':'css selector','value':selector})[ELEMENT]
        command('POST', '/actions', {'actions':[{'type':'pointer','id':'help-mouse',
            'parameters':{'pointerType':'mouse'},'actions':[{'type':'pointerMove',
            'duration':0,'origin':{ELEMENT:element},'x':0,'y':0}]}]})
    badge = '[aria-label="עזרה: תיקית מקור"]'
    visible = "return !document.querySelector('#context-help').hidden"
    hover('h1')
    hover(badge)
    wait(lambda: script(visible))
    capture('01-native-help-hover')
    # Also enforce synchronous dismissal in a single renderer turn.
    assert script("const b=document.querySelector('[aria-label=\"עזרה: תיקית מקור\"]');b.dispatchEvent(new PointerEvent('pointerleave',{pointerType:'mouse'}));return document.querySelector('#context-help').hidden")
    hover('h1')
    hover(badge)
    click(badge)
    click(badge)
    hover('h1')
    assert not script(visible), 'Mouse clicks must not pin help after pointer leave'
    script("document.querySelector('#source').focus();document.querySelector('[aria-label=\"עזרה: תיקית מקור\"]').focus()")
    assert script(visible)
    # Native key input, not a dispatched keyboard event.
    element = command('POST', '/element', {'using':'css selector','value':badge})[ELEMENT]
    command('POST', f'/element/{element}/value', {'text':'\ue00c'})  # Escape
    assert not script(visible)
    command('POST', f'/element/{element}/value', {'text':'\ue007'})  # Enter
    assert script(visible)
    command('POST', f'/element/{element}/value', {'text':'\ue004'})  # Tab
    assert not script(visible)
    click('#menu')
    click('[data-help="about"]')
    assert script("return document.querySelector('#usage-content').hidden && !document.querySelector('#about-content').hidden")
    assert script("return document.querySelector('#about-version').textContent") == json.loads((root/'package.json').read_text())['version']
    assert script("return document.querySelector('[data-help=about]').getAttribute('aria-pressed')") == 'true'
    capture('01-native-about')
    click('.about-credits summary')
    assert script("return document.querySelector('.about-credits').open")
    assert script("const d=document.querySelector('#menu-dialog');return d.scrollWidth<=d.clientWidth+1")
    capture('01-native-about-credits')
    assert script("return !document.querySelector('#menu-dialog #developer-mode')"), 'Developer mode belongs to Settings'
    check_phone_layout(command, script, capture, output, '01-native-about')
    click('#menu-dialog .close-dialog')
    assert script("return document.activeElement.id==='menu'")
    click('#open-settings')
    assert script("return document.querySelector('#advanced-dialog #developer-mode') !== null")
    assert not script("return document.querySelector('#developer-mode').checked")
    check_phone_layout(command, script, capture, output, '01-native-display-settings')
    click('#advanced-dialog .close-dialog')
    assert script("return document.activeElement.id==='open-settings'")
    command('DELETE', '/actions')
    return ['Actual pointer leave closes help immediately after hover and repeated clicks',
            'Actual keyboard help focus, Escape, Enter and Tab dismissal',
            'About shows packaged version, source lineage and expandable credits without horizontal overflow',
            'Developer mode is in advanced Settings, accessible from the global settings button']


def set_developer_mode(script, click, wait, enabled):
    click('#open-settings')
    if script("return document.querySelector('#developer-mode').checked") != enabled:
        click('#developer-mode')
    wait(lambda: script("return document.querySelector('#app').dataset.developerMode") == str(enabled).lower())
    click('#advanced-dialog .close-dialog')
