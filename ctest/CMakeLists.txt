cmake_minimum_required(VERSION 3.10)
set (CMAKE_CXX_STANDARD 11)
project(cwarp2)
add_executable(cwarp2 main.cxx)
target_include_directories(cwarp2 PUBLIC /opt/homebrew/include)
target_include_directories(cwarp2 PUBLIC ../)
target_link_directories(cwarp2 PUBLIC ../target/release)
target_link_libraries(cwarp2 warp2)
