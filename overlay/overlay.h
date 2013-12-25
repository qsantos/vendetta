#ifndef O_OVERLAY_H
#define O_OVERLAY_H

typedef struct overlay overlay_t;

#include "../game.h"
#include "swbuilding.h"
#include "swinventory.h"
#include "swskills.h"

struct overlay
{
	kindOf_building_t* selectedBuilding;

	swbuilding_t  swbuilding;
	swinventory_t swinventory;
	swskills_t    swskills;
};

overlay_t* overlay_init(void);
void       overlay_exit(overlay_t* o);

void draw_buildPanel(game_t* g);
void draw_cursor    (game_t* g);
void draw_overlay   (game_t* g);

int overlay_catch(game_t* g, float x, float y, int t);

#endif
