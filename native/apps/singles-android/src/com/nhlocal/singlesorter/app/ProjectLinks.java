package com.nhlocal.singlesorter.app;

/** Only explicit About actions may request these fixed external project pages. */
final class ProjectLinks {
    private ProjectLinks() {}
    static String resolve(String target) {
        if ("Source".equals(target)) return "https://github.com/NHLOCAL/Singles-Sorter";
        if ("License".equals(target)) return "https://github.com/NHLOCAL/Singles-Sorter/blob/main/LICENSE";
        throw new IllegalArgumentException("PROJECT_LINK_TARGET");
    }
}
