/// ONLINE
// World: The name of the world object
with(World){
hbuffer_clear(__ONLINE_buffer);
hbuffer_write_uint16(__ONLINE_buffer, __ONLINE_socket);
hbuffer_write_uint16(__ONLINE_buffer, __ONLINE_udpsocket);
hbuffer_write_string(__ONLINE_buffer, __ONLINE_selfID);
hbuffer_write_string(__ONLINE_buffer, __ONLINE_name);
hbuffer_write_string(__ONLINE_buffer, __ONLINE_selfGameID);
hbuffer_write_uint8(__ONLINE_buffer, __ONLINE_race);
__ONLINE_n = instance_number(__ONLINE_onlinePlayer);
hbuffer_write_uint16(__ONLINE_buffer, __ONLINE_n);
hbuffer_write_uint16(__ONLINE_buffer, __ONLINE_vis);
for(__ONLINE_i = 0; __ONLINE_i < __ONLINE_n; __ONLINE_i += 1){
__ONLINE_oPlayer = instance_find(__ONLINE_onlinePlayer, __ONLINE_i);
hbuffer_write_string(__ONLINE_buffer, __ONLINE_oPlayer.__ONLINE_ID);
hbuffer_write_int32(__ONLINE_buffer, __ONLINE_oPlayer.x);
hbuffer_write_int32(__ONLINE_buffer, __ONLINE_oPlayer.y);
hbuffer_write_int32(__ONLINE_buffer, __ONLINE_oPlayer.sprite_index);
hbuffer_write_float32(__ONLINE_buffer, __ONLINE_oPlayer.image_speed);
hbuffer_write_float32(__ONLINE_buffer, __ONLINE_oPlayer.image_xscale);
hbuffer_write_float32(__ONLINE_buffer, __ONLINE_oPlayer.image_yscale);
hbuffer_write_float32(__ONLINE_buffer, __ONLINE_oPlayer.image_angle);
hbuffer_write_uint16(__ONLINE_buffer, __ONLINE_oPlayer.__ONLINE_oRoom);
hbuffer_write_string(__ONLINE_buffer, __ONLINE_oPlayer.__ONLINE_name);
}
hbuffer_write_to_file(__ONLINE_buffer, "tempOnline");
}
/// ONLINE
// World: The name of the world object
// Player: The name of the player object
// : The name of the player2 object if it exists
if(file_exists("tempOnline2")){
hbuffer_clear(World.__ONLINE_buffer);
hbuffer_read_from_file(World.__ONLINE_buffer, "tempOnline2");
World.__ONLINE_sGravity = hbuffer_read_uint8(World.__ONLINE_buffer);
World.__ONLINE_sX = hbuffer_read_int32(World.__ONLINE_buffer);
World.__ONLINE_sY = hbuffer_read_float64(World.__ONLINE_buffer);
World.__ONLINE_sRoom = hbuffer_read_int16(World.__ONLINE_buffer);
file_delete("tempOnline2");
if(room_exists(World.__ONLINE_sRoom)){
__ONLINE_p = Player;
global.grav = __ONLINE_sGravity;
__ONLINE_p = Player;
__ONLINE_p.x = World.__ONLINE_sX;
__ONLINE_p.y = World.__ONLINE_sY;
room_goto(World.__ONLINE_sRoom);
}
World.__ONLINE_sSaved = false;
}
/// ONLINE
// World: The name of the world object
// Player: The name of the player object
// : The name of the player2 object if it exists
if(file_exists("tempOnline2")){
hbuffer_clear(World.__ONLINE_buffer);
hbuffer_read_from_file(World.__ONLINE_buffer, "tempOnline2");
World.__ONLINE_sGravity = hbuffer_read_uint8(World.__ONLINE_buffer);
World.__ONLINE_sX = hbuffer_read_int32(World.__ONLINE_buffer);
World.__ONLINE_sY = hbuffer_read_float64(World.__ONLINE_buffer);
World.__ONLINE_sRoom = hbuffer_read_int16(World.__ONLINE_buffer);
file_delete("tempOnline2");
if(room_exists(World.__ONLINE_sRoom)){
__ONLINE_p = Player;
global.grav = __ONLINE_sGravity;
__ONLINE_p = Player;
__ONLINE_p.x = World.__ONLINE_sX;
__ONLINE_p.y = World.__ONLINE_sY;
room_goto(World.__ONLINE_sRoom);
}
World.__ONLINE_sSaved = false;
}