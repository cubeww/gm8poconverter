/// ONLINE
// World: The name of the world object
// Player: The name of the player object
// : The name of the player2 object if it exists
if(!World.__ONLINE_race){
if(room != rmInit && room != rmTitle && room != rmMenu && room != rmOptions){
hbuffer_clear(World.__ONLINE_buffer);
__ONLINE_p = Player;
if(instance_exists(__ONLINE_p)){
hbuffer_write_uint8(World.__ONLINE_buffer, 5);

hbuffer_write_uint8(World.__ONLINE_buffer, global.grav);
hbuffer_write_int32(World.__ONLINE_buffer, __ONLINE_p.x);
hbuffer_write_float64(World.__ONLINE_buffer, __ONLINE_p.y);
hbuffer_write_int16(World.__ONLINE_buffer, room);
hsocket_write_message(World.__ONLINE_socket, World.__ONLINE_buffer);
}
}
}