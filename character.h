#ifndef CHARACTER_H
#define CHARACTER_H

#include "object.h"

typedef struct character character_t;

struct character
{
	object_t o;

	float go_x;
	float go_y;
	object_t* go_o;
};

void character_init   (character_t* c);
void character_doRound(character_t* c, float duration);

#endif
