# Standard CMake FetchContent acquisition; no product runtime network dependency.
include(FetchContent)

# Validate the completed acquisition again at the caller before any source copy.
function(sorter_require_acquired_source name source actual_digest expected_digest)
  # Reject foreign/root spellings before touching their filesystem or a UNC host.
  set(expected_source "${CMAKE_BINARY_DIR}/_deps/sorter_${name}-src")
  if("${source}" STREQUAL "" OR NOT IS_ABSOLUTE "${source}" OR
      NOT source STREQUAL expected_source)
    message(FATAL_ERROR "Dependency acquisition did not return its expected source path: ${name}")
  endif()
  if(NOT IS_DIRECTORY "${source}" OR NOT EXISTS "${source}/CMakeLists.txt")
    message(FATAL_ERROR "Dependency acquisition did not return a valid source directory: ${name}")
  endif()
  file(REAL_PATH "${CMAKE_BINARY_DIR}" build_root)
  file(REAL_PATH "${source}" source_root)
  set(expected_root "${build_root}/_deps/sorter_${name}-src")
  if(NOT source_root STREQUAL expected_root OR
      "${actual_digest}" STREQUAL "" OR NOT actual_digest STREQUAL expected_digest)
    message(FATAL_ERROR "Dependency acquisition source identity mismatch: ${name}")
  endif()
endfunction()

function(sorter_acquire_dependency name version url sha256 result)
  # Failure must not leave a stale parent-scope result from an earlier call.
  set(${result} "" PARENT_SCOPE)
  set(${result}_archive_sha256 "" PARENT_SCOPE)
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
  # file(MAKE_DIRECTORY) can report a nonfatal configure error and unwind this
  # function. An explicit process result makes failure stop the entire configure.
  execute_process(COMMAND "${CMAKE_COMMAND}" -E make_directory "${SORTER_DEPENDENCY_CACHE}"
    RESULT_VARIABLE mkdir_result ERROR_VARIABLE mkdir_error)
  if(NOT mkdir_result STREQUAL "0")
    message(FATAL_ERROR "Cannot create dependency cache: ${SORTER_DEPENDENCY_CACHE}: ${mkdir_error}")
  endif()
  # A source archive cache can be shared by different target/CRT build directories.
  # Serialize population, including the download, without a second downloader.
  file(LOCK "${SORTER_DEPENDENCY_CACHE}/acquire.lock" GUARD FUNCTION TIMEOUT 180 RESULT_VARIABLE lock_result)
  if(NOT lock_result STREQUAL "0")
    message(FATAL_ERROR "Cannot lock dependency cache: ${lock_result}")
  endif()
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
    SOURCE_DIR "${CMAKE_BINARY_DIR}/_deps/sorter_${name}-src"
    BINARY_DIR "${CMAKE_BINARY_DIR}/_deps/sorter_${name}-build"
    SUBBUILD_DIR "${CMAKE_BINARY_DIR}/_deps/sorter_${name}-subbuild"
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
  file(SHA256 "${archive}" acquired_digest)
  if(NOT acquired_digest STREQUAL sha256)
    message(FATAL_ERROR "Dependency archive changed during acquisition: ${name}")
  endif()
  sorter_require_acquired_source("${name}" "${sorter_${name}_SOURCE_DIR}" "${acquired_digest}" "${sha256}")
  set(${result} "${sorter_${name}_SOURCE_DIR}" PARENT_SCOPE)
  set(${result}_archive_sha256 "${acquired_digest}" PARENT_SCOPE)
endfunction()
