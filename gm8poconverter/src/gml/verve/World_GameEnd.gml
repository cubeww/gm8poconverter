/// ONLINE
if(!file_exists("temp") && !file_exists(working_directory+"\save\temp") && !file_exists("temp.dat")){
if(file_exists("tempOnline")){
file_delete("tempOnline");
}
if(file_exists("tempOnline2")){
file_delete("tempOnline2");
}
}
hbuffer_destroy(__ONLINE_buffer);
if(!file_exists("tempOnline")){
hsocket_destroy(__ONLINE_socket);
hudpsocket_destroy(__ONLINE_udpsocket);
}