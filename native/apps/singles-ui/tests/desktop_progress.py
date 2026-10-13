"""Small, source-bound checkpoints for the real CI acceptance process."""
import json
import time


class DesktopProgress:
    def __init__(self, path, binding, clock=time.monotonic):
        self.path, self.clock = path, clock
        self.started = self.phase_started = clock()
        self.state = dict(schema=1, status='running', binding=dict(binding),
                          active_phase=None, active_case=None, last_action=None,
                          active_wait=None, completed_phases=[])
        self.write()

    def write(self):
        now = self.clock()
        self.state['elapsed_seconds'] = round(now - self.started, 3)
        self.state['phase_elapsed_seconds'] = round(now - self.phase_started, 3)
        pending = self.path.with_suffix('.pending')
        pending.write_text(json.dumps(self.state, ensure_ascii=False, indent=2), encoding='utf-8')
        pending.replace(self.path)

    def phase(self, name):
        if self.state['active_phase'] is not None:
            self.state['completed_phases'].append(dict(
                phase=self.state['active_phase'], elapsed_seconds=round(self.clock() - self.phase_started, 3)))
        self.phase_started = self.clock()
        self.state.update(active_phase=name, active_case=None, last_action=None, active_wait=None)
        self.write()
        print('DESKTOP_PHASE ' + name, flush=True)

    def action(self, kind, detail, case=None):
        self.state['last_action'] = dict(kind=kind, detail=str(detail),
                                         started_seconds=round(self.clock() - self.started, 3))
        if case is not None:
            self.state['active_case'] = str(case)
        self.write()

    def waiting(self, timeout):
        self.state['active_wait'] = dict(timeout_seconds=timeout,
                                        started_seconds=round(self.clock() - self.started, 3))
        self.write()

    def waited(self):
        self.state['active_wait'] = None
        self.write()

    def finish(self, passed, error=None):
        if passed and self.state['active_phase'] is not None:
            self.state['completed_phases'].append(dict(
                phase=self.state['active_phase'], elapsed_seconds=round(self.clock() - self.phase_started, 3)))
            self.state['active_phase'] = None
        self.state['status'] = 'passed' if passed else 'failed'
        if error is not None:
            self.state['error'] = str(error)[:1000]
        self.write()
