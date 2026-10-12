import copy
import unittest
from desktop_stability import assert_shell


def valid():
    return dict(width=360,height=800,pageWidth=360,pageHeight=800,
                footer=dict(x=0,y=766,right=360,bottom=800,width=360,height=34),
                main=dict(bottom=766),footerControls=[dict(x=150,y=768,right=200,bottom=798)])


class StableShellTests(unittest.TestCase):
    def test_bounded_shell_accepts_stable_footer(self):
        assert_shell(valid(),valid())

    def test_footer_gap_is_rejected(self):
        value=valid();value['footer']['bottom']=770
        with self.assertRaises(AssertionError):assert_shell(value)

    def test_notification_must_not_change_footer_height(self):
        value=valid();value['footer']['height']=44;value['footer']['y']=756;value['main']['bottom']=756
        with self.assertRaises(AssertionError):assert_shell(value,valid())

    def test_page_and_control_horizontal_overflow_are_rejected(self):
        for key in ('pageWidth','pageHeight'):
            value=valid();value[key]+=20
            with self.assertRaises(AssertionError):assert_shell(value)
        value=valid();value['footerControls'][0]['right']=380
        with self.assertRaises(AssertionError):assert_shell(value)

    def test_scroll_content_cannot_obscure_footer(self):
        value=valid();value['main']['bottom']=780
        with self.assertRaises(AssertionError):assert_shell(value)


if __name__=='__main__':unittest.main()
