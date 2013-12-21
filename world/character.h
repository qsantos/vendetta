#ifndef W_CHARACTER_H
#define W_CHARACTER_H

#include "object.h"
#include "../universe/universe.h"

typedef struct character character_t;

typedef enum
{
	D_NORTH,
	D_EAST,
	D_SOUTH,
	D_WEST,
} direction_t;

struct character
{
	object_t o;

	float       go_x;
	float       go_y;
	object_t*   go_o;
	direction_t dir;

	float* materials;
};

void character_init   (character_t* c, universe_t* u);
void character_deinit (character_t* c);
void character_doRound(character_t* c, float duration);

#endif
