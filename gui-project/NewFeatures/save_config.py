import json

class SoftwareConfigurator:
    def __init__(self, config_file='config.json'):
        self.config_file = config_file
        self._load_settings()

    def _load_settings(self):
        try:
            with open(self.config_file, 'r') as f:
                self.user_settings = json.load(f)
        except FileNotFoundError:
            self.user_settings = {}

    def save_settings(self):
        with open(self.config_file, 'w') as f:
            json.dump(self.user_settings, f, indent=4)

    # ... More methods to handle individual settings can go here
    
    
    

    

if __name__ == '__main__':
    
    
    c = SoftwareConfigurator('config.json')
    c.