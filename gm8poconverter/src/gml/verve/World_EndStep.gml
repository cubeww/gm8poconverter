/// ONLINE
// Player: The name of the player object
// : The name of the player2 object if it exists
// TCP SOCKETS
hsocket_update_read(__ONLINE_socket);
while(hsocket_read_message(__ONLINE_socket, __ONLINE_buffer)){
switch(hbuffer_read_uint8(__ONLINE_buffer)){
case 0:
// CREATED
__ONLINE_ID = hbuffer_read_string(__ONLINE_buffer);
__ONLINE_found = false;
for(__ONLINE_i = 0; __ONLINE_i < instance_number(__ONLINE_onlinePlayer) && !__ONLINE_found; __ONLINE_i += 1){
if(instance_find(__ONLINE_onlinePlayer, __ONLINE_i).__ONLINE_ID == __ONLINE_ID){
__ONLINE_found = true;
}
}
if(!__ONLINE_found){
__ONLINE_oPlayer = instance_create(0, 0, __ONLINE_onlinePlayer);
__ONLINE_oPlayer.__ONLINE_ID = __ONLINE_ID;
__ONLINE_oPlayer.__ONLINE_name = hbuffer_read_string(__ONLINE_buffer);
}
break;
case 1:
// DESTROYED
__ONLINE_ID = hbuffer_read_string(__ONLINE_buffer);
__ONLINE_found = false;
for(__ONLINE_i = 0; __ONLINE_i < instance_number(__ONLINE_onlinePlayer) && !__ONLINE_found; __ONLINE_i += 1){
__ONLINE_oPlayer = instance_find(__ONLINE_onlinePlayer, __ONLINE_i);
if(__ONLINE_oPlayer.__ONLINE_ID == __ONLINE_ID){
with(__ONLINE_oPlayer){
instance_destroy();
}
__ONLINE_found = true;
}
}
break;
case 2:
// INCOMPATIBLE VERSION
__ONLINE_lastVersion = hbuffer_read_string(__ONLINE_buffer);
__ONLINE_errorMessage = "Your tool uses the version "+__ONLINE_version+" but the oldest compatible version is "+__ONLINE_lastVersion+". Please update your tool.";
wd_message_simple(__ONLINE_errorMessage);
game_end();
exit;
break;
case 4:
// CHAT MESSAGE
__ONLINE_ID = hbuffer_read_string(__ONLINE_buffer);
__ONLINE_found = false;
__ONLINE_oPlayer = 0;
for(__ONLINE_i = 0; __ONLINE_i < instance_number(__ONLINE_onlinePlayer) && !__ONLINE_found; __ONLINE_i += 1){
__ONLINE_oPlayer = instance_find(__ONLINE_onlinePlayer, __ONLINE_i);
if(__ONLINE_oPlayer.__ONLINE_ID == __ONLINE_ID){
__ONLINE_found = true;
}
}
if(__ONLINE_found){
__ONLINE_message = hbuffer_read_string(__ONLINE_buffer);
__ONLINE_oChatbox = instance_create(0, 0, __ONLINE_chatbox);
__ONLINE_oChatbox.__ONLINE_message = __ONLINE_message;
__ONLINE_oChatbox.__ONLINE_follower = __ONLINE_oPlayer;
if(__ONLINE_oPlayer.visible){
sound_play(__ONLINE_sndChatbox);
}
}
break;
case 5:
// SOMEONE SAVED
if(!__ONLINE_race){
__ONLINE_sSaved = true;
__ONLINE_sGravity = hbuffer_read_uint8(__ONLINE_buffer);
__ONLINE_sName = hbuffer_read_string(__ONLINE_buffer);
__ONLINE_sX = hbuffer_read_int32(__ONLINE_buffer);
__ONLINE_sY = hbuffer_read_float64(__ONLINE_buffer);
__ONLINE_sRoom = hbuffer_read_int16(__ONLINE_buffer);
__ONLINE_a = instance_create(0, 0, __ONLINE_playerSaved);
__ONLINE_a.__ONLINE_name = __ONLINE_sName;
__ONLINE_a.__ONLINE_state = -1;
hbuffer_clear(__ONLINE_buffer);
hbuffer_write_uint8(__ONLINE_buffer, __ONLINE_sGravity);
hbuffer_write_int32(__ONLINE_buffer, __ONLINE_sX);
hbuffer_write_float64(__ONLINE_buffer, __ONLINE_sY);
hbuffer_write_int16(__ONLINE_buffer, __ONLINE_sRoom);
hbuffer_write_to_file(__ONLINE_buffer, "tempOnline2");
sound_play(__ONLINE_sndSaved);
}
break;
case 6:
// SELF ID
__ONLINE_selfID = hbuffer_read_string(__ONLINE_buffer);
break;
}
}
__ONLINE_mustQuit = false;
switch(hsocket_get_state(__ONLINE_socket)){
case 2:
if(!__ONLINE_connected){
__ONLINE_connected = true;
}
break;
case 4:
wd_message_simple("Connection closed.");
__ONLINE_mustQuit = true;
break;
case 5:
hsocket_reset(__ONLINE_socket);
__ONLINE_errorMessage = "Could not connect to the server.";
if(__ONLINE_connected){
__ONLINE_errorMessage = "Connection lost";
}
wd_message_simple(__ONLINE_errorMessage);
__ONLINE_mustQuit = true;
break;
}
if(__ONLINE_mustQuit){
if(file_exists("temp")){
file_delete("temp");
}
game_end();
exit;
}
__ONLINE_p = Player;
__ONLINE_exists = instance_exists(__ONLINE_p);
__ONLINE_X = __ONLINE_pX;
__ONLINE_Y = __ONLINE_pY;
if(__ONLINE_exists){
if(__ONLINE_exists != __ONLINE_pExists){
// SEND PLAYER CREATE
hbuffer_clear(__ONLINE_buffer);
hbuffer_write_uint8(__ONLINE_buffer, 0);
hsocket_write_message(__ONLINE_socket, __ONLINE_buffer);
}
__ONLINE_X = __ONLINE_p.x;
__ONLINE_Y = __ONLINE_p.y;
__ONLINE_stoppedFrames += 1;
if(__ONLINE_pX != __ONLINE_X || __ONLINE_pY != __ONLINE_Y || keyboard_check_released(vk_anykey) || keyboard_check_pressed(vk_anykey)){
__ONLINE_stoppedFrames = 0;
}
if(__ONLINE_stoppedFrames < 5 || __ONLINE_t < 3){
if(__ONLINE_t >= 3){
__ONLINE_t = 0;
}
// SEND PLAYER MOVED
if(__ONLINE_selfID != ""){
hbuffer_clear(__ONLINE_buffer);
hbuffer_write_uint8(__ONLINE_buffer, 1);
hbuffer_write_string(__ONLINE_buffer, __ONLINE_selfID);
hbuffer_write_string(__ONLINE_buffer, __ONLINE_selfGameID);
hbuffer_write_uint16(__ONLINE_buffer, room);
hbuffer_write_uint64(__ONLINE_buffer, current_time);
hbuffer_write_int32(__ONLINE_buffer, __ONLINE_X);
hbuffer_write_int32(__ONLINE_buffer, __ONLINE_Y);
hbuffer_write_int32(__ONLINE_buffer, __ONLINE_p.sprite_index);
hbuffer_write_float32(__ONLINE_buffer, __ONLINE_p.image_speed);
hbuffer_write_float32(__ONLINE_buffer, __ONLINE_p.image_xscale * __ONLINE_p.x_scale);
hbuffer_write_float32(__ONLINE_buffer, __ONLINE_p.image_yscale * global.grav);
hbuffer_write_float32(__ONLINE_buffer, __ONLINE_p.image_angle);
hbuffer_write_string(__ONLINE_buffer, __ONLINE_name);
hudpsocket_send(__ONLINE_udpsocket, __ONLINE_buffer);
}
}
__ONLINE_t += 1;
if(keyboard_check_pressed(vk_space)){
__ONLINE_message = wd_input_box("Chat", "Say something:", "");
__ONLINE_message = string_replace_all(__ONLINE_message, "#", "\\#");
__ONLINE_message_length = string_length(__ONLINE_message);
if(__ONLINE_message_length > 0){
__ONLINE_message_max_length = 300;
if(__ONLINE_message_length > __ONLINE_message_max_length){
__ONLINE_message = string_copy(__ONLINE_message, 0, __ONLINE_message_max_length);
}
hbuffer_clear(__ONLINE_buffer);
hbuffer_write_uint8(__ONLINE_buffer, 4);
hbuffer_write_string(__ONLINE_buffer, __ONLINE_message);
hsocket_write_message(__ONLINE_socket, __ONLINE_buffer);
__ONLINE_oChatbox = instance_create(0, 0, __ONLINE_chatbox);
__ONLINE_oChatbox.__ONLINE_message = __ONLINE_message;
__ONLINE_oChatbox.__ONLINE_follower = __ONLINE_p;
sound_play(__ONLINE_sndChatbox);
}
}
}else{
if(__ONLINE_exists != __ONLINE_pExists){
// SEND PLAYER DESTROYED
hbuffer_clear(__ONLINE_buffer);
hbuffer_write_uint8(__ONLINE_buffer, 1);
hsocket_write_message(__ONLINE_socket, __ONLINE_buffer);
}
}
__ONLINE_pExists = __ONLINE_exists;
__ONLINE_pX = __ONLINE_X;
__ONLINE_pY = __ONLINE_Y;
__ONLINE_heartbeat += 1/room_speed;
if(__ONLINE_heartbeat > 3){
__ONLINE_heartbeat = 0;
// SEND PLAYER HEARTBEAT
hbuffer_clear(__ONLINE_buffer);
hbuffer_write_uint8(__ONLINE_buffer, 2);

hsocket_write_message(__ONLINE_socket, __ONLINE_buffer);
}
hsocket_update_write(__ONLINE_socket);
// UDP SOCKETS
while(hudpsocket_receive(__ONLINE_udpsocket, __ONLINE_buffer)){
switch(hbuffer_read_uint8(__ONLINE_buffer)){
case 1:
// RECEIVED MOVED
__ONLINE_ID = hbuffer_read_string(__ONLINE_buffer);
__ONLINE_gameID = hbuffer_read_string(__ONLINE_buffer);
__ONLINE_found = false;
__ONLINE_oPlayer = 0;
for(__ONLINE_i = 0; __ONLINE_i < instance_number(__ONLINE_onlinePlayer) && !__ONLINE_found; __ONLINE_i += 1){
__ONLINE_oPlayer = instance_find(__ONLINE_onlinePlayer, __ONLINE_i);
if(__ONLINE_oPlayer.__ONLINE_ID == __ONLINE_ID){
__ONLINE_found = true;
}
}
if(!__ONLINE_found){
__ONLINE_oPlayer = instance_create(0, 0, __ONLINE_onlinePlayer);
__ONLINE_oPlayer.__ONLINE_ID = __ONLINE_ID;
}
__ONLINE_oPlayer.__ONLINE_oRoom = hbuffer_read_uint16(__ONLINE_buffer);
__ONLINE_syncTime = hbuffer_read_uint64(__ONLINE_buffer);
if(__ONLINE_oPlayer.__ONLINE_syncTime < __ONLINE_syncTime){
__ONLINE_oPlayer.__ONLINE_syncTime = __ONLINE_syncTime;
__ONLINE_oPlayer.x = hbuffer_read_int32(__ONLINE_buffer);
__ONLINE_oPlayer.y = hbuffer_read_int32(__ONLINE_buffer);
__ONLINE_oPlayer.sprite_index = hbuffer_read_int32(__ONLINE_buffer);
__ONLINE_oPlayer.image_speed = hbuffer_read_float32(__ONLINE_buffer);
__ONLINE_oPlayer.image_xscale = hbuffer_read_float32(__ONLINE_buffer);
__ONLINE_oPlayer.image_yscale = hbuffer_read_float32(__ONLINE_buffer);
__ONLINE_oPlayer.image_angle = hbuffer_read_float32(__ONLINE_buffer);
__ONLINE_oPlayer.__ONLINE_name = hbuffer_read_string(__ONLINE_buffer);
}
break;
default:
wd_message_simple("Received unexpected data from the server.");
}
}
if(hudpsocket_get_state(__ONLINE_udpsocket) != 1){
wd_message_simple("Connection to the UDP socket lost.");
game_end();
exit;
}
if(keyboard_check_pressed(ord('V'))){
if(__ONLINE_vis == 0) __ONLINE_vis = 1;
else if(__ONLINE_vis == 1) __ONLINE_vis = 2;
else if(__ONLINE_vis == 2) __ONLINE_vis = 0;
__ONLINE_a = instance_create(0, 0, __ONLINE_playerSaved);
__ONLINE_a.__ONLINE_state = __ONLINE_vis;
}