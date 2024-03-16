/// ONLINE
// Player: The name of the player object
// : The name of the player2 object if it exists
visible = __ONLINE_oRoom == room;
image_alpha = __ONLINE_alpha;
__ONLINE_p = Player;
if(instance_exists(__ONLINE_p)){
__ONLINE_dist = distance_to_object(__ONLINE_p);
image_alpha = min(__ONLINE_alpha, __ONLINE_dist/100);
}