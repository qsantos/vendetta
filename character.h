#ifndef CHARACTER_H
#define CHARACTER_H

typedef struct character character_t;

struct character
{
	float x;
	float y;

	float go_x;
	float go_y;
};

void character_doRound(character_t* c, float duration);

#endif
