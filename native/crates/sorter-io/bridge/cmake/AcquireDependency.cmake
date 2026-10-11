# Standard CMake FetchContent acquisition; no product runtime network dependency.
include(FetchContent)

function(sorter_acquire_dependency name version url sha256 result)
  if(NOT DEFINED SORTER_DEPENDENCY_CACHE OR SORTER_DEPENDENCY_CACHE STREQUAL "")
    if(DEFINED ENV{SORTER_DEPENDENCY_CACHE} AND NOT "$ENV{SORTER_DEPENDENCY_CACHE}" STREQUAL "")
      set(SORTER_DEPENDENCY_CACHE "$ENV{SORTER_DEPENDENCY_CACHE}")
    elseif(WIN32 AND DEFINED ENV{LOCALAPPDATA})
      set(SORTER_DEPENDENCY_CACHE "$ENV{LOCALAPPDATA}/SinglesSorter/cache/sources")
    elseif(DEFINED ENV{XDG_CACHE_HOME})
      set(SORTER_DEPENDENCY_CACHE "$ENV{XDG_CACHE_HOME}/singles-sorter/sources")
    elseif(DEFINED ENV{HOME})
      set(SORTER_DEPENDENCY_CACHE "$ENV{HOME}/.cache/singles-sorter/sources")
    else()
      set(SORTER_DEPENDENCY_CACHE "${CMAKE_BINARY_DIR}/downloads")
    endif()
  endif()
  # Native Windows paths from -D or environment variables must be converted
  # before FetchContent serializes them into generated CMake scripts. SET handles
  # one host path (including Windows long-path syntax), not a search-path list.
  cmake_path(SET SORTER_DEPENDENCY_CACHE "${SORTER_DEPENDENCY_CACHE}")
  if(NOT IS_ABSOLUTE "${SORTER_DEPENDENCY_CACHE}")
    message(FATAL_ERROR "SORTER_DEPENDENCY_CACHE must be an absolute path")
  endif()
  if(NOT DEFINED SORTER_DEPENDENCIES_OFFLINE)
    set(SORTER_DEPENDENCIES_OFFLINE "$ENV{SORTER_DEPENDENCIES_OFFLINE}")
  endif()
  string(TOUPPER "sorter_${name}" upper)
  if(DEFINED FETCHCONTENT_SOURCE_DIR_${upper} AND NOT "${FETCHCONTENT_SOURCE_DIR_${upper}}" STREQUAL "")
    message(FATAL_ERROR "Unverified source override is unsupported; import the official archive into SORTER_DEPENDENCY_CACHE instead")
  endif()
  file(MAKE_DIRECTORY "${SORTER_DEPENDENCY_CACHE}")
  # A source archive cache can be shared by different target/CRT build directories.
  # Serialize population, including the download, without a second downloader.
  file(LOCK "${SORTER_DEPENDENCY_CACHE}/acquire.lock" GUARD FUNCTION TIMEOUT 180)
  set(archive "${SORTER_DEPENDENCY_CACHE}/${name}-${version}.tar.gz")
  if(EXISTS "${archive}")
    file(SHA256 "${archive}" actual)
    if(NOT actual STREQUAL sha256)
      message(FATAL_ERROR "Dependency SHA256 mismatch: ${archive}. Remove or replace this cache file with the official archive; it was not overwritten.")
    endif()
    if(SORTER_DEPENDENCIES_OFFLINE)
      set(location "${archive}")
    else()
      # Keep the declared URL stable across warm online builds. URL_HASH makes
      # FetchContent reuse this verified DOWNLOAD_DIR file without downloading.
      set(location "${url}")
    endif()
  elseif(SORTER_DEPENDENCIES_OFFLINE)
    message(FATAL_ERROR "Dependency offline cache is missing: ${archive}. Download ${url} on an allowed connection, verify SHA256 ${sha256}, and place it at this path.")
  else()
    set(location "${url}")
  endif()
  # Offline is enforced by selecting a verified local archive. CMake's generic
  # FULLY_DISCONNECTED option cannot populate a fresh build tree on its own.
  set(FETCHCONTENT_FULLY_DISCONNECTED OFF)
  set(FETCHCONTENT_QUIET OFF)
  set(FETCHCONTENT_TRY_FIND_PACKAGE_MODE NEVER)
  FetchContent_Declare(sorter_${name}
    URL "${location}"
    URL_HASH "SHA256=${sha256}"
    DOWNLOAD_DIR "${SORTER_DEPENDENCY_CACHE}"
    DOWNLOAD_NAME "${name}-${version}.tar.gz"
    TLS_VERIFY TRUE
    TIMEOUT 120
    INACTIVITY_TIMEOUT 30
    DOWNLOAD_EXTRACT_TIMESTAMP FALSE
    # Build integration is explicit below, after configuring our exact options.
    SOURCE_SUBDIR _sorter_population_only)
  FetchContent_MakeAvailable(sorter_${name})
  set(${result} "${sorter_${name}_SOURCE_DIR}" PARENT_SCOPE)
endfunction()
