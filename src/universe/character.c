#include "character.h"

void kindOf_character_init(kindOf_character_t* c)
{
	c->sprite = -1;
}

void kindOf_character_exit(kindOf_character_t* c)
{
	(void) c;
}
