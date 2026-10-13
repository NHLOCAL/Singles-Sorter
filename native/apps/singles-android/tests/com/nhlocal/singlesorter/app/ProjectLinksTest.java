package com.nhlocal.singlesorter.app;

public final class ProjectLinksTest {
    private ProjectLinksTest() {}
    public static void main(String[] args) {
        if (!ProjectLinks.resolve("Source").equals("https://github.com/NHLOCAL/Singles-Sorter")) throw new AssertionError("source");
        if (!ProjectLinks.resolve("License").equals("https://github.com/NHLOCAL/Singles-Sorter/blob/main/LICENSE")) throw new AssertionError("license");
        String[] invalid = {null, "", "source", "License ", "https://example.com", "javascript:alert(1)", "file:///private"};
        for (String value : invalid) {
            try { ProjectLinks.resolve(value); throw new AssertionError("unapproved target"); }
            catch (IllegalArgumentException expected) { /* fail closed */ }
        }
        System.out.println("9 fixed project link checks passed");
    }
}
