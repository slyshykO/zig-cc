
if(CMAKE_GENERATOR MATCHES "Visual Studio")
    message(FATAL_ERROR "Visual Studio generator not supported, use: cmake -G Ninja")
endif()

IF (WIN32)
    SET(TOOL_EXECUTABLE_SUFFIX ".exe")
ELSE()
    SET(TOOL_EXECUTABLE_SUFFIX "")
ENDIF()

set(CMAKE_C_COMPILER   zig-cc${TOOL_EXECUTABLE_SUFFIX})
set(CMAKE_CXX_COMPILER zig-cpp${TOOL_EXECUTABLE_SUFFIX})
set(CMAKE_ASM_COMPILER ${CMAKE_C_COMPILER})
set(CMAKE_AR           zig-ar${TOOL_EXECUTABLE_SUFFIX})
set(CMAKE_RANLIB       zig-ranlib${TOOL_EXECUTABLE_SUFFIX})

SET(CMAKE_OBJDUMP      "zig-objdump${TOOL_EXECUTABLE_SUFFIX}" CACHE INTERNAL "objdump tool")
SET(CMAKE_CPPFILT      "zig-c++filt${TOOL_EXECUTABLE_SUFFIX}" CACHE INTERNAL "C++filt")
set(CMAKE_OBJCOPY      "zig-objcopy${TOOL_EXECUTABLE_SUFFIX}" CACHE INTERNAL "objcopy tool")
set(CMAKE_SIZE_UTIL    "zig-size${TOOL_EXECUTABLE_SUFFIX}"    CACHE INTERNAL "size tool")

SET(CMAKE_C_FLAGS_DEBUG               "-Og -g" CACHE INTERNAL "c compiler flags debug")
SET(CMAKE_CXX_FLAGS_DEBUG             "-Og -g" CACHE INTERNAL "cxx compiler flags debug")
SET(CMAKE_ASM_FLAGS_DEBUG             "-g" CACHE INTERNAL "asm compiler flags debug")
SET(CMAKE_EXE_LINKER_FLAGS_DEBUG      "" CACHE INTERNAL "linker flags debug")

SET(CMAKE_C_FLAGS_RELEASE             "-O3 -DNDEBUG" CACHE INTERNAL "c compiler flags release")
SET(CMAKE_CXX_FLAGS_RELEASE           "-O3 -DNDEBUG" CACHE INTERNAL "cxx compiler flags release")
SET(CMAKE_ASM_FLAGS_RELEASE           "" CACHE INTERNAL "asm compiler flags release")
SET(CMAKE_EXE_LINKER_FLAGS_RELEASE    "" CACHE INTERNAL "linker flags release")

SET(CMAKE_C_FLAGS_MINSIZEREL          "-Os -DNDEBUG" CACHE INTERNAL "c compiler flags release")
SET(CMAKE_CXX_FLAGS_MINSIZEREL        "-Os -DNDEBUG" CACHE INTERNAL "cxx compiler flags release")
SET(CMAKE_ASM_FLAGS_MINSIZEREL        "" CACHE INTERNAL "asm compiler flags release")
SET(CMAKE_EXE_LINKER_FLAGS_MINSIZEREL "" CACHE INTERNAL "linker flags release")

SET(CMAKE_C_FLAGS_RELWITHDEBINFO           "-O2 -g -DNDEBUG" CACHE INTERNAL "c compiler flags release")
SET(CMAKE_CXX_FLAGS_RELWITHDEBINFO         "-O2 -g -DNDEBUG" CACHE INTERNAL "cxx compiler flags release")
SET(CMAKE_ASM_FLAGS_RELWITHDEBINFO         "-g" CACHE INTERNAL "asm compiler flags release")
SET(CMAKE_EXE_LINKER_FLAGS_RELWITHDEBINFO  "" CACHE INTERNAL "linker flags release")