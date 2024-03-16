/// ONLINE
// Player: The name of the player object
// : The name of the player2 object if it exists
__ONLINE_f = __ONLINE_follower;
if(instance_exists(__ONLINE_f)){
x = __ONLINE_f.x;
y = __ONLINE_f.y;
}else{
instance_destroy();
exit;
}
if(__ONLINE_fade){
__ONLINE_fadeAlpha -= 0.02;
if(__ONLINE_fadeAlpha <= 0){
instance_destroy();
exit;
}
}
__ONLINE_alpha = 1;
if(__ONLINE_follower != Player){
visible = __ONLINE_follower.visible;
__ONLINE_p = Player;
if(instance_exists(__ONLINE_p)){
__ONLINE_dist = distance_to_object(__ONLINE_p);
__ONLINE_alpha = __ONLINE_dist/100;
}
}
__ONLINE_t -= 1;
if(__ONLINE_t < 0){
__ONLINE_fade = true;
}
// Destroy all other chatboxes of the same player
if(!__ONLINE_hasDestroyed){
__ONLINE_found = false;
__ONLINE_oChatbox = 0;
for(__ONLINE_i = 0; __ONLINE_i < instance_number(__ONLINE_chatbox) && !__ONLINE_found; __ONLINE_i += 1){
__ONLINE_oChatbox = instance_find(__ONLINE_chatbox, __ONLINE_i);
if(__ONLINE_oChatbox.__ONLINE_follower == __ONLINE_follower && __ONLINE_oChatbox.id != id){
__ONLINE_found = true;
}
}
if(__ONLINE_found){
with(__ONLINE_oChatbox){
instance_destroy();
}
}
__ONLINE_hasDestroyed = true;
}