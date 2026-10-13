/* Owned-device build QA only; never packaged in the product APK.
 * dlopen/dlclose execute the exact JNI library's constructors/destructors. */
#include <dlfcn.h>
#include <stdio.h>
int main(int argc, char **argv) {
    if (argc != 2) { fputs("one owned JNI library path required\n", stderr); return 2; }
    void *library = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (library == NULL) { fprintf(stderr, "dlopen: %s\n", dlerror()); return 1; }
    if (dlsym(library, "Java_com_nhlocal_singlesorter_inference_OriginalPlanning_plan0") == NULL) {
        fputs("product JNI entry missing\n", stderr); dlclose(library); return 1;
    }
    if (dlclose(library) != 0) { fputs("dlclose failed\n", stderr); return 1; }
    puts("ORIGINAL_PRODUCT_JNI_INITIALIZERS_OK");
    return 0;
}
