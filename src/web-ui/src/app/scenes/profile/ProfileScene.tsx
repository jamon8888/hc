import React from 'react';
import { NurseryView } from './views';
import './ProfileScene.scss';

const ProfileScene: React.FC = () => (
  <div className="alex-profile-scene" data-alex-scene="profile" data-alex-part="root">
    <NurseryView />
  </div>
);

export default ProfileScene;
