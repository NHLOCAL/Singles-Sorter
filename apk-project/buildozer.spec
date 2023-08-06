[app]

# (str) Title of your application
title = The Order of Singles

# (str) Package name
package.name = order-of-singles

# (str) Package domain (needed for android/ios packaging)
package.domain = nhlocal.github.io

# (str) Version of your application
version = 1.0

# (str) Custom source folders for requirements (Separate multiple entries with a comma)
source.include_exts = py,png,jpg,kv,csv,atlas

# (list) Application requirements
requirements = python3,kivy

# (str) Icon filename
#icon.filename = path/to/your/icon.png

# (str) Supported orientation (landscape, portrait or all)
orientation = portrait

# (str) Short description of the application
description = Your app description here.

# (list) Permissions
android.permissions = INTERNET

# (int) Presplash background color (b, g, r, a)
presplash.color = 0, 0, 0, 1

# (str) Presplash image (a default one is provided)
# presplash.filename = presplash.png

# (str) App source code directory
source.dir = .

# (list) List of source files
source.include_patterns = assets/*, *.py

# (str) Log level (0 = error only, 1 = info, 2 = debug (with command line python -m buildozer))
log_level = 1

# (str) Android NDK version to use
android.ndk = 21.1.6352462

# (bool) Use a black overlay under the statusbar (android)
android.blacklist = armeabi-v7a, x86

# (str) iOS SDK version to use
ios.sdk = 14.4

# (list) iOS icons (space separated filenames)
# ios.icons = Icon-40.png Icon-58.png Icon-80.png Icon-87.png Icon-120.png Icon-152.png Icon-167.png Icon-180.png

# (str) iOS launch image (portrait)
# ios.launch_image = landscape.png
