/// ONLINE
// d41d8cd98f00b204e9800998ecf8427e: The ID of the game
// 81.70.53.71: The server
// 8002: The TCP port
// 8003: The UDP port
// The 'Needle': The game name
// 1.1.9: The version
hhttp_dll_init(0)
__ONLINE_connected = false;
__ONLINE_buffer = hbuffer_create();
__ONLINE_selfID = "";
__ONLINE_name = "";
__ONLINE_selfGameID = "$GAME_ID";
__ONLINE_server = "$SERVER_IP";
__ONLINE_version = "1.1.9";
__ONLINE_race = false;
__ONLINE_vis = 0;
if(file_exists("tempOnline")){
hbuffer_read_from_file(__ONLINE_buffer, "tempOnline");
__ONLINE_socket = hbuffer_read_uint16(__ONLINE_buffer);
__ONLINE_udpsocket = hbuffer_read_uint16(__ONLINE_buffer);
__ONLINE_selfID = hbuffer_read_string(__ONLINE_buffer);
__ONLINE_name = hbuffer_read_string(__ONLINE_buffer);
__ONLINE_selfGameID = hbuffer_read_string(__ONLINE_buffer);
__ONLINE_race = hbuffer_read_uint8(__ONLINE_buffer);
__ONLINE_n = hbuffer_read_uint16(__ONLINE_buffer);
__ONLINE_vis = hbuffer_read_uint16(__ONLINE_buffer);
for(__ONLINE_i = 0; __ONLINE_i < __ONLINE_n; __ONLINE_i += 1){
__ONLINE_oPlayer = instance_create(0, 0, __ONLINE_onlinePlayer);
__ONLINE_oPlayer.__ONLINE_ID = hbuffer_read_string(__ONLINE_buffer);
__ONLINE_oPlayer.x = hbuffer_read_int32(__ONLINE_buffer);
__ONLINE_oPlayer.y = hbuffer_read_int32(__ONLINE_buffer);
__ONLINE_oPlayer.sprite_index = hbuffer_read_int32(__ONLINE_buffer);
__ONLINE_oPlayer.image_speed = hbuffer_read_float32(__ONLINE_buffer);
__ONLINE_oPlayer.image_xscale = hbuffer_read_float32(__ONLINE_buffer);
__ONLINE_oPlayer.image_yscale = hbuffer_read_float32(__ONLINE_buffer);
__ONLINE_oPlayer.image_angle = hbuffer_read_float32(__ONLINE_buffer);
__ONLINE_oPlayer.__ONLINE_oRoom = hbuffer_read_uint16(__ONLINE_buffer);
__ONLINE_oPlayer.__ONLINE_name = hbuffer_read_string(__ONLINE_buffer);
}
}else{
__ONLINE_socket = hsocket_create();
hsocket_connect(__ONLINE_socket, __ONLINE_server, 8002);
__ONLINE_name = wd_input_box("Name", "Enter your name:", "");
if(__ONLINE_name == ""){
__ONLINE_name = "Anonymous";
}
__ONLINE_name = string_replace_all(__ONLINE_name, "#", "\#");
if(string_length(__ONLINE_name) > 20){
__ONLINE_name = string_copy(__ONLINE_name, 0, 20);
}
__ONLINE_password = wd_input_box("Password", "Leave it empty for no password:", "");
if(string_length(__ONLINE_password) > 20){
__ONLINE_password = string_copy(__ONLINE_password, 0, 20);
}
__ONLINE_selfGameID += __ONLINE_password;
wd_message_set_text("Do you want to enable RACE mod? (shared saves will be disabled)");
__ONLINE_race = wd_message_show(wd_mk_information, wd_mb_yes, wd_mb_no, 0) == wd_mb_yes;
hbuffer_clear(__ONLINE_buffer);
hbuffer_write_uint8(__ONLINE_buffer, 3);
hbuffer_write_string(__ONLINE_buffer, __ONLINE_name);
hbuffer_write_string(__ONLINE_buffer, __ONLINE_selfGameID);
hbuffer_write_string(__ONLINE_buffer, "$GAME_NAME");
hbuffer_write_string(__ONLINE_buffer, __ONLINE_version);
hbuffer_write_uint8(__ONLINE_buffer, __ONLINE_password != "");
hsocket_write_message(__ONLINE_socket, __ONLINE_buffer);
__ONLINE_udpsocket = hudpsocket_create();
hudpsocket_start(__ONLINE_udpsocket, false, 0);
hudpsocket_set_destination(__ONLINE_udpsocket, __ONLINE_server, 8003);
hbuffer_clear(__ONLINE_buffer);
hbuffer_write_uint8(__ONLINE_buffer, 0);
hudpsocket_send(__ONLINE_udpsocket, __ONLINE_buffer);
}
__ONLINE_pExists = false;
__ONLINE_pX = 0;
__ONLINE_pY = 0;
__ONLINE_t = 0;
__ONLINE_heartbeat = 0;
__ONLINE_stoppedFrames = 0;
__ONLINE_sGravity = 0;
__ONLINE_sX = 0;
__ONLINE_sY = 0;
__ONLINE_sRoom = 0;
__ONLINE_sSaved = false;
sound_add_included("__ONLINE_sndChatbox.wav", 0, 1)
sound_add_included("__ONLINE_sndSaved.wav", 0, 1)
globalvar __ONLINE_sndChatbox, __ONLINE_sndSaved;
__ONLINE_sndChatbox = "__ONLINE_sndChatbox"
__ONLINE_sndSaved = "__ONLINE_sndSaved"
