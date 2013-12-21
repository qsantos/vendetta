#include "universe.h"

void universe_init(universe_t* u)
{
	u->materials[0].name = "Stone";
	u->mines[0].name = "Mine";
	u->mines[0].material = &u->materials[0];
}
