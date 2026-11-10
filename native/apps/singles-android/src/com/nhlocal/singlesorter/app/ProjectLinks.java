package com.nhlocal.singlesorter.app;

/** Only explicit About actions may request these fixed external project pages. */
final class ProjectLinks {
    private ProjectLinks() {}
    static String resolve(String target) {
        if ("Website".equals(target)) return "https://singles-sorter.ze-kal.top";
        if ("Feedback".equals(target)) return "https://docs.google.com/forms/d/e/1FAIpQLScOaX1wWW1YXXlX4cylMA6LWpO7yIb2fStmjzfSqmLc_V9CIw/viewform?usp=sf_link";
        if ("Source".equals(target)) return "https://github.com/NHLOCAL/Singles-Sorter";
        if ("License".equals(target)) return "https://github.com/NHLOCAL/Singles-Sorter/blob/main/LICENSE";
        throw new IllegalArgumentException("PROJECT_LINK_TARGET");
    }
}
